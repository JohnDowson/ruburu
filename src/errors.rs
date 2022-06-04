use std::io::Cursor;

use rocket::{
    http::ContentType,
    response::{self, Responder},
    Request, Response,
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("{0}")]
    Db(#[from] sqlx::Error),
    #[error("{0}")]
    Rocket(#[from] rocket::Error),
    #[error("{0}")]
    Dotenv(#[from] dotenv::Error),
    #[error("404: NotFound")]
    NotFound,
}

impl<'r, 'o: 'r> Responder<'r, 'o> for Error {
    fn respond_to(self, _: &'r Request<'_>) -> response::Result<'o> {
        let f = format!("{self}");
        Response::build()
            .header(ContentType::HTML)
            .sized_body(f.len(), Cursor::new(f))
            .ok()
    }
}
