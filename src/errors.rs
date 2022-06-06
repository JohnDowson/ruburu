use std::io::Cursor;

use rocket::{
    http::{ContentType, Status},
    response::{self, Responder},
    Request, Response,
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("{0}")]
    Db(#[from] sqlx::Error),
    #[error("{0}")]
    Image(#[from] image::ImageError),
    #[error("{0}")]
    Rocket(#[from] rocket::Error),
    #[error("{0}")]
    Dotenv(#[from] dotenv::Error),
    #[error("404: NotFound")]
    NotFound,
    #[error("{0}")]
    Io(#[from] std::io::Error),
    #[error("Banned. Reason: {0}")]
    Banned(String),
    #[error("You must supply an image when creating a thread")]
    MissingImage,
    #[error("You're supposed to have a captcha cookie to do that")]
    MissingOrInvalidCaptchaID,
}

impl<'r, 'o: 'r> Responder<'r, 'o> for Error {
    fn respond_to(self, _: &'r Request<'_>) -> response::Result<'o> {
        let status = match self {
            Error::Db(_) => Status::InternalServerError,
            Error::Image(_) => Status::InternalServerError,
            Error::Rocket(_) => Status::InternalServerError,
            Error::Dotenv(_) => Status::InternalServerError,
            Error::NotFound => Status::NotFound,
            Error::Io(_) => Status::InternalServerError,
            Error::Banned(_) => Status::Ok,
            Error::MissingImage => Status::UnprocessableEntity,
            Error::MissingOrInvalidCaptchaID => Status::UnprocessableEntity,
        };
        let f = format!("{self}");
        Response::build()
            .header(ContentType::HTML)
            .status(status)
            .sized_body(f.len(), Cursor::new(f))
            .ok()
    }
}
