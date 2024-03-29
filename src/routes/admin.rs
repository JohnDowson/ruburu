use super::public;
use crate::{
    errors::Error,
    models::{AdminPrivilege, Board, BoardForm, LoginForm},
};
use maud::{html, Markup};
use rocket::{form::Form, get, post, response::Redirect, uri, State};
use sqlx::PgPool;

#[get("/admin")]
pub async fn index(pool: &State<PgPool>, privilege: AdminPrivilege) -> Result<Markup, Error> {
    Ok(html! {
        head {
            link rel="stylesheet" href="/static/style.css";
        }
        body {
            h1 { (format!("Hello {}", privilege.uid())) }
        }
    })
}

#[get("/admin", rank = 2)]
pub async fn login_page(pool: &State<PgPool>) -> Result<Markup, Error> {
    Ok(html! {
        head {
            link rel="stylesheet" href="/static/style.css";
        }
        body {
            h1 { "Hello, ruburu!" }
            div {
                form id="board" action=(uri!(create_board).to_string()) method="post" {
                    label for="name" { "Name" }
                    input type="text" name="name";br;
                    label for="title" { "Title" }
                    input type="text" name="title";br;
                    input type="submit";
                }
            }
        }
    })
}
#[post("/admin/login", data = "<form>")]
pub async fn login(
    pool: &State<PgPool>,
    form: Form<LoginForm<'_>>,
    _privilege: AdminPrivilege,
) -> Result<Redirect, Error> {
    Ok(Redirect::to(uri!(index)))
}

#[post("/admin/submit", data = "<form>")]
pub async fn create_board(
    pool: &State<PgPool>,
    form: Form<BoardForm<'_>>,
    _privilege: AdminPrivilege,
) -> Result<Redirect, Error> {
    let form = form.into_inner();
    Board::create(form.name.as_ref(), form.title.as_ref(), pool).await?;
    Ok(Redirect::to(uri!(public::board(form.name.as_ref()))))
}
