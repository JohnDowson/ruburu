use std::net::IpAddr;

use crate::errors::Error;
use crate::models::{Board, Image, NotBanned, Post, PostForm};
use maud::{html, Markup};
use rocket::form::Form;
use rocket::response::Redirect;
use rocket::{get, post, uri, State};
use sqlx::PgPool;

#[get("/")]
pub async fn index(pool: &State<PgPool>) -> Result<Markup, Error> {
    Ok(html! {
        head {
            link rel="stylesheet" href="/static/style.css";
        }
        body {
            h1 { "Hello, ruburu!" }
            div {
                @for board in Board::get_all(&*pool).await? {
                    div { a href=(uri!(board(board.name())).to_string()) { (board.name()) } }
                }
            }
        }
    })
}

#[post("/submit", data = "<form>")]
pub async fn create_post(
    form: Form<PostForm<'_>>,
    pool: &State<PgPool>,
    ip: IpAddr,
    _not_banned: NotBanned,
) -> Result<Redirect, Error> {
    let image = if let Some(file) = Some(&form.image) {
        Some(Image::from_buf(&*file, &*pool).await?)
    } else {
        None
    };
    let id = if let Some(thread) = form.thread {
        Post::create(
            form.board.as_ref(),
            thread,
            form.title.as_deref(),
            form.author.as_deref(),
            form.email.as_deref(),
            form.sage,
            form.content.as_deref(),
            ip.into(),
            image,
            &*pool,
        )
        .await?;
        thread
    } else {
        Post::create_thread(
            form.board.as_ref(),
            form.title.as_deref(),
            form.author.as_deref(),
            form.email.as_deref(),
            form.sage,
            form.content.as_deref(),
            ip.into(),
            {
                if let Some(image) = image {
                    image
                } else {
                    return Err(Error::MissingImage);
                }
            },
            &*pool,
        )
        .await?
    };
    Ok(Redirect::to(uri!(thread(&*form.board, id))))
}

#[get("/<board>")]
pub async fn board(board: &str, pool: &State<PgPool>) -> Result<Markup, Error> {
    let board = Board::get(board, &*pool).await?.ok_or(Error::NotFound)?;
    Ok(html! {
        head {
            link rel="stylesheet" href="/static/style.css";
        }
        body {
            h1 { (board.name()) }
            h2 { (board.title()) }
            (post_form(board.name(), None))
            @for head in Post::threads_for_board(board.name(), &*pool).await? {
                (post_body(&head, &*pool).await?)
            }
        }
    })
}

#[get("/<board>/<thread>")]
pub async fn thread(board: &str, thread: i32, pool: &State<PgPool>) -> Result<Markup, Error> {
    let board = Board::get(board, &*pool).await?.ok_or(Error::NotFound)?;
    let posts = Post::for_thread(board.name(), thread, &*pool).await?;
    Ok(html! {
        head {
            link rel="stylesheet" href="/static/style.css";
        }
        body {
            h1 { (board.name()) }
            h2 { (board.title()) }
            (post_form(board.name(), Some(thread)))
            .thread {
                @for post in posts {
                    (post_body(&post, &*pool).await?)
                }
            }
        }
    })
}

async fn post_body(post: &Post, pool: &PgPool) -> Result<Markup, Error> {
    Ok(html! {
        .post id=(post.id()) {
            .info {
                @if post.sage() {
                    .sage { ("⇓") }
                }
                @if let Some(title) = post.title() {
                    .title { (title) }
                }
                @if let Some(author) = post.author() {
                    .author { (author) }
                }
                @if let Some(email) = post.email() {
                    .email { (email) }
                }
                .id {
                    a href=(format!("{}#{}", uri!(thread(post.board(), post.thread())), post.id())) { (">>") }
                    a href="" { (post.id()) }
                }
                .timestamp { (post.posted_at().format("%Y-%m-%d %H:%M:%S")) }
            }
            .content {
                @if let Some(img) = post.image() {
                    a href=(format!("/images/{}", img)) {
                        img src=(format!("/thumbs/{}.png", img));
                    }
                };
                (post.html_content())
            }
            .replies {
                @for reply in post.replies(pool)
                                  .await?
                                  .into_iter()
                                  .map(|r| html! {
                                      a href=(format!("{}#{}", uri!(thread(r.board(), r.thread())), r.id())) { (">>")(r.id()) }
                                    })
                                  .intersperse(maud::PreEscaped(", ".to_string())) {
                    (reply)
                }
            }
        }
    })
}

fn post_form(board: &str, thread: Option<i32>) -> Markup {
    html! {
        .post-form {
            form id="post" action=(uri!(create_post).to_string()) method="post" enctype="multipart/form-data" {
                table {
                    tbody {
                        tr {
                            td { label for="author" { "Name" } }
                            td { input type="text" name="author"; }
                        }
                        tr {
                            td { label for="title" { "Title" } }
                            td { input type="text" name="title"; }
                        }
                        tr {
                            td { label for="email" { "Email" }  }
                            td { input type="text" name="email";  }
                        }
                        tr {
                            td { label for="image" { "Image" }  }
                            td { input type="file" name="image" accept="image/png, image/jpeg";  }
                        }
                        tr {
                            td { label for="sage" { "Sage" } }
                            td {
                                input type="checkbox" name="sage";
                                input type="submit";
                            }
                        }
                        tr {
                            td { label for="content" { "Content" } }
                            td { textarea name="content" form="post" {} }
                        }
                    }
                    input type="hidden" name="board" value=(board);
                    @if let Some(thread) = thread {
                        input type="hidden" name="thread" value=(thread);
                    }
                }
            }
        }
    }
}
