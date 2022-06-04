use crate::errors::Error;
use crate::models::{Board, Post, PostForm};
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
pub async fn create_post(form: Form<PostForm>, pool: &State<PgPool>) -> Result<Redirect, Error> {
    let id = if let Some(thread) = form.thread {
        Post::create(
            form.board.as_ref(),
            thread,
            form.title.as_deref(),
            form.author.as_deref(),
            form.email.as_deref(),
            form.sage,
            form.content.as_deref(),
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
            &*pool,
        )
        .await?
    };
    Ok(Redirect::to(uri!(thread(&form.board, id))))
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
                .post {
                    @if let Some(title) = head.title() {
                        h3 { (title) }
                    }
                    span { a href=(uri!(thread(board.name(), head.id())).to_string()){ (head.id()) } }
                    div { (head.content().unwrap_or("")) }
                }
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
                    (post_body(&post))
                }
            }
        }
    })
}

fn post_body(post: &Post) -> Markup {
    html! {
        .post {
            @if let Some(title) = post.title() {
                h3 { (title) }
            }
            div {
                span { (post.id()) }
                br;
                span { (post.posted_at().to_string()) }
            }
            div { (post.content().unwrap_or("")) }
        }
    }
}

fn post_form(board: &str, thread: Option<i32>) -> Markup {
    html! {
        div {
            form id="post" action=(uri!(create_post).to_string()) method="post" {
                label for="author" { "Name" }
                input type="text" name="author";br;
                label for="title" { "Title" }
                input type="text" name="title";br;
                label for="sage" { "Sage" }
                input type="checkbox" name="sage";br;
                input type="hidden" name="board" value=(board);
                @if let Some(thread) = thread {
                    input type="hidden" name="thread" value=(thread);
                }
                input type="submit";
            }
            textarea name="content" form="post" {}
        }
    }
}
