#![feature(iter_intersperse)]

mod errors;
mod fairings;
mod models;
mod routes;

use crate::{errors::Error, routes::*};
use rocket::{fs::FileServer, routes};

#[rocket::main]
async fn main() -> Result<(), Error> {
    dotenv::dotenv()?;
    let _rocket = rocket::build()
        .attach(fairings::DbManager)
        .mount("/static", FileServer::from("./static"))
        .mount("/thumbs", FileServer::from("./thumbs"))
        .mount("/images", FileServer::from("./images"))
        .mount(
            "/",
            routes![
                public::index,
                public::board,
                public::thread,
                public::create_post,
                admin::index,
                admin::login_page,
                admin::login,
                admin::create_board
            ],
        )
        .launch()
        .await?;

    Ok(())
}
