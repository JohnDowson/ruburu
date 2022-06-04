use rocket::{
    fairing::{self, Fairing, Info, Kind},
    Build, Rocket,
};
use std::{env, future::Future, pin::Pin};

pub(crate) struct DbManager;

impl Fairing for DbManager {
    fn info(&self) -> Info {
        Info {
            name: "DbManager",
            kind: Kind::Singleton | Kind::Ignite,
        }
    }

    fn on_ignite<'life0, 'async_trait>(
        &'life0 self,
        rocket: Rocket<Build>,
    ) -> Pin<Box<dyn Future<Output = fairing::Result> + Send + 'async_trait>>
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async {
            let db_uri: String = env::var("DATABASE_URL").expect("Please configure DATABASE_URL");
            let pool = sqlx::PgPool::connect(&db_uri)
                .await
                .expect("Couldn't create DB pool");
            sqlx::migrate!("./migrations")
                .run(&pool)
                .await
                .expect("Couldn't run migrations");
            Ok(rocket.manage(pool))
        })
    }
}
