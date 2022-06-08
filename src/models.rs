use image::ImageEncoder;
use maud::{html, PreEscaped};
use once_cell::sync::Lazy;
use rand::prelude::StdRng;
use regex::{Captures, Regex};
use rocket::{
    async_trait,
    data::ToByteUnit,
    form::FromFormField,
    http::Status,
    request::{self, FromRequest},
    uri, FromForm, Request,
};
use sqlx::{
    query, query_as,
    types::{ipnetwork::IpNetwork, time::PrimitiveDateTime, uuid::Uuid},
    PgPool, Postgres,
};
use std::ops::Deref;
use tokio::io::AsyncWriteExt;

use crate::errors::Error;

static REPLY_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"&gt;&gt;(\d+)").unwrap());
static BOLD_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(\*\*)(.+?)(\*\*)").unwrap());
static ITALIC_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(\*)(.+?)(\*)").unwrap());

pub struct Board {
    name: String,
    title: String,
}

impl Board {
    pub async fn get_all(pool: &PgPool) -> Result<Vec<Board>, sqlx::Error> {
        query_as!(Board, "SELECT name, title FROM boards ORDER BY name")
            .fetch_all(pool)
            .await
    }

    pub async fn get(name: &str, pool: &PgPool) -> Result<Option<Board>, sqlx::Error> {
        query_as!(
            Board,
            "SELECT name, title FROM boards WHERE name = $1",
            name
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn create(name: &str, title: &str, pool: &PgPool) -> Result<(), sqlx::Error> {
        query!(
            "INSERT INTO boards(name, title)
                VALUES ($1, $2)",
            name,
            title
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Get a reference to the board's name.
    #[must_use]
    pub fn name(&self) -> &str {
        self.name.as_ref()
    }

    /// Get a reference to the board's title.
    #[must_use]
    pub fn title(&self) -> &str {
        self.title.as_ref()
    }
}

pub struct Post {
    id: i32,
    board: String,
    title: Option<String>,
    author: Option<String>,
    email: Option<String>,
    sage: bool,
    plaintext_content: Option<String>,
    html_content: String,
    posted_at: PrimitiveDateTime,
    thread: i32,
    ip: IpNetwork,
    image: Option<Uuid>,
}

impl Post {
    pub async fn for_thread(board: &str, id: i32, pool: &PgPool) -> Result<Vec<Post>, Error> {
        let res = query_as!(
            Post,
            "SELECT * FROM posts WHERE thread = $1 AND board = $2",
            id,
            board
        )
        .fetch_all(pool)
        .await?;
        if res.is_empty() {
            Err(Error::NotFound)
        } else {
            Ok(res)
        }
    }

    pub async fn threads_for_board(board: &str, pool: &PgPool) -> Result<Vec<Post>, sqlx::Error> {
        query_as!(
            Post,
            "WITH threads AS (
                SELECT DISTINCT ON (posts.thread) posts.thread as id, max(posts.posted_at) as last_post
                FROM posts
                WHERE posts.board = $1 AND (posts.thread = posts.id OR NOT posts.sage)
                GROUP BY posts.thread
            )
            SELECT posts.*
            FROM posts
                LEFT JOIN threads ON posts.thread = threads.id
            WHERE posts.id = threads.id
            ORDER BY threads.last_post DESC",
            board
        )
        .fetch_all(pool)
        .await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn create_thread(
        board: &str,
        title: Option<&str>,
        author: Option<&str>,
        email: Option<&str>,
        sage: bool,
        content: Option<&str>,
        ip: IpNetwork,
        image: Image,
        pool: &PgPool,
    ) -> Result<i32, sqlx::Error> {
        let mut tx = pool.begin().await?;
        let per_board_id = query!(
            "UPDATE boards
            SET next_post_id = next_post_id + 1
            WHERE name = $1
            RETURNING next_post_id;",
            board
        )
        .fetch_one(&mut tx)
        .await?
        .next_post_id;

        let (html_content, replied) = Post::html_body(content, board, pool).await?;

        query!(
            "INSERT INTO posts(id, board, title, author, email, sage, plaintext_content, html_content, thread, ip, image)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $1, $9, $10)
            RETURNING id;",
            per_board_id,
            board,
            title,
            author,
            email,
            sage,
            content,
            html_content,
            ip,
            image.hash()
        )
        .fetch_one(&mut tx)
        .await?;

        for message in replied {
            query!(
                "INSERT INTO replies(message_id, message_board, reply_id, reply_board, reply_thread)
                VALUES ($1, $2, $3, $2, $3);",
                message,
                board,
                per_board_id
            )
            .execute(pool)
            .await?;
        }

        tx.commit().await?;

        Ok(per_board_id)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn create(
        board: &str,
        thread: i32,
        title: Option<&str>,
        author: Option<&str>,
        email: Option<&str>,
        sage: bool,
        content: Option<&str>,
        ip: IpNetwork,
        image: Option<Image>,
        pool: &PgPool,
    ) -> Result<i32, sqlx::Error> {
        let mut tx = pool.begin().await?;
        let per_board_id = query!(
            "UPDATE boards
            SET next_post_id = next_post_id + 1
            WHERE name = $1
            RETURNING next_post_id;",
            board
        )
        .fetch_one(&mut tx)
        .await?
        .next_post_id;

        let (html_content, replied) = Post::html_body(content, board, pool).await?;

        query!(
            "INSERT INTO posts(id, board, title, author, email, sage, plaintext_content, html_content, thread, ip, image)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9,  $10, $11);",
            per_board_id,
            board,
            title,
            author,
            email,
            sage,
            content,
            html_content,
            thread,
            ip,
            image.map(|i|i.hash())
        )
        .execute(pool)
        .await?;

        for message in replied {
            query!(
                "INSERT INTO replies(message_id, message_board, reply_id, reply_board, reply_thread)
                VALUES ($1, $2, $3, $2, $4);",
                message,
                board,
                per_board_id,
                thread
            )
            .execute(pool)
            .await?;
        }
        tx.commit().await?;
        Ok(per_board_id)
    }

    /// Get the post's replies.
    pub async fn replies(&self, pool: &PgPool) -> Result<Vec<Reply>, sqlx::Error> {
        query_as!(
            Reply,
            "SELECT reply_id, reply_board, reply_thread
            FROM replies
            WHERE message_id = $1 AND message_board = $2",
            self.id,
            self.board
        )
        .fetch_all(pool)
        .await
    }

    async fn html_body(
        body: Option<&str>,
        board: &str,
        pool: &PgPool,
    ) -> Result<(String, Vec<i32>), sqlx::Error> {
        if let Some(body) = body {
            let body = html! {
                @for line in body.lines() {
                    @if line.starts_with('>') && line.chars().nth(1) != Some('>') {
                        .green-text { (line) }
                    } @else { (line) }
                    br;
                }
            }
            .0;

            let body = BOLD_RE.replace_all(&*body, |c: &Captures| format!(r"<b>{}</b>", &c[2]));
            let body = ITALIC_RE.replace_all(&*body, |c: &Captures| format!(r"<em>{}</em>", &c[2]));
            let replied: Vec<i32> = REPLY_RE
                .captures_iter(&*body)
                .map(|c| c[1].parse().unwrap())
                .collect();

            let replied = query!(
                "SELECT id, thread
                        FROM posts
                        WHERE id = ANY($1) AND board = $2",
                &replied,
                board
            )
            .fetch_all(pool)
            .await?;

            let body = REPLY_RE.replace_all(&*body, |c: &Captures| {
                let id: i32 = c[1].parse().unwrap();
                if let Some(r) = replied.iter().find(|r| r.id == id) {
                    format!(
                        r#"<a href="{}#{}">&gt;&gt;{}</a>"#,
                        uri!(crate::routes::public::thread(board, r.thread)),
                        &c[1],
                        &c[1]
                    )
                } else {
                    format!(r#"&gt;&gt;{}"#, &c[1])
                }
            });
            Ok((
                body.into_owned(),
                replied.into_iter().map(|r| r.id).collect(),
            ))
        } else {
            Ok((
                html! {
                    .post-content {}
                }
                .0,
                Vec::new(),
            ))
        }
    }

    /// Get the post's id.
    #[must_use]
    pub fn id(&self) -> i32 {
        self.id
    }

    /// Get a reference to the post's rendered content.
    #[must_use]
    pub fn html_content(&self) -> PreEscaped<&str> {
        PreEscaped(self.html_content.as_ref())
    }

    /// Get a reference to the post's title.
    #[must_use]
    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    /// Get a reference to the post's author.
    #[must_use]
    pub fn author(&self) -> Option<&str> {
        self.author.as_deref()
    }

    /// Get a reference to the post's email.
    #[must_use]
    pub fn email(&self) -> Option<&str> {
        self.email.as_deref()
    }

    pub fn posted_at(&self) -> &PrimitiveDateTime {
        &self.posted_at
    }

    pub fn board(&self) -> &str {
        self.board.as_ref()
    }

    pub fn thread(&self) -> i32 {
        self.thread
    }

    pub fn sage(&self) -> bool {
        self.sage
    }

    pub fn image(&self) -> Option<&Uuid> {
        self.image.as_ref()
    }
}

pub struct Reply {
    reply_id: i32,
    reply_board: String,
    reply_thread: i32,
}

impl Reply {
    /// Get the reply's id.
    #[must_use]
    pub fn id(&self) -> i32 {
        self.reply_id
    }

    /// Get a reference to the reply's board.
    #[must_use]
    pub fn board(&self) -> &str {
        self.reply_board.as_ref()
    }

    /// Get the reply's thread.
    #[must_use]
    pub fn thread(&self) -> i32 {
        self.reply_thread
    }
}

pub struct NonEmptyStr<'s>(&'s str);

impl<'s> std::fmt::Debug for NonEmptyStr<'s> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl<'s> std::fmt::Display for NonEmptyStr<'s> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[async_trait]
impl<'v> FromFormField<'v> for NonEmptyStr<'v> {
    fn from_value(field: rocket::form::ValueField<'v>) -> rocket::form::Result<'v, Self> {
        let str = <&'v str as FromFormField<'v>>::from_value(field)?;
        if str.is_empty() {
            let error = rocket::form::Error::validation("Empty NonEmptyStr");
            Err(error.into())
        } else {
            Ok(Self(str))
        }
    }
}

impl<'s> AsRef<str> for NonEmptyStr<'s> {
    fn as_ref(&self) -> &str {
        self.0
    }
}

impl<'s> Deref for NonEmptyStr<'s> {
    type Target = str;
    fn deref(&self) -> &str {
        self.0
    }
}

pub struct Image {
    hash: Uuid,
}

impl Image {
    pub async fn from_buf(buf: &[u8], pool: &PgPool) -> Result<Image, Error> {
        let hash = {
            let hash = md5::compute(buf);
            Uuid::from_bytes(hash.0)
        };
        let maybe = query!(
            r#"SELECT CASE WHEN EXISTS (
                SELECT hash FROM images WHERE hash = $1
            ) THEN TRUE ELSE FALSE END as "exits!""#,
            hash
        )
        .fetch_one(pool)
        .await?
        .exits;
        if maybe {
            Ok(Image { hash })
        } else {
            let mut file = tokio::fs::File::create(format!("./images/{}", hash)).await?;
            file.write_all(buf).await?;

            let image = image::load_from_memory(buf)?;
            let image = image.resize(200, 200, image::imageops::FilterType::Lanczos3);
            let mut buf = Vec::new();
            let encoder = image::codecs::png::PngEncoder::new(&mut buf);
            encoder.write_image(
                image.as_bytes(),
                image.width(),
                image.height(),
                image.color(),
            )?;

            let mut file = tokio::fs::File::create(format!("./thumbs/{}.png", hash)).await?;
            file.write_all(&buf).await?;

            query!("INSERT INTO images VALUES ($1)", hash)
                .execute(pool)
                .await?;
            Ok(Image { hash })
        }
    }

    pub fn hash(&self) -> Uuid {
        self.hash
    }

    pub fn uri(&self) -> String {
        format!("/images/{}", self.hash)
    }
}

#[derive(FromForm, Debug)]
pub struct PostForm<'r> {
    pub title: Option<NonEmptyStr<'r>>,
    pub author: Option<NonEmptyStr<'r>>,
    pub email: Option<NonEmptyStr<'r>>,
    pub sage: bool,
    pub content: Option<NonEmptyStr<'r>>,
    pub thread: Option<i32>,
    pub board: NonEmptyStr<'r>,
    pub image: Option<Bytes>,
    pub captcha: Option<NonEmptyStr<'r>>,
}

impl<'r> PostForm<'r> {
    pub fn captcha(&self) -> Option<&str> {
        self.captcha.as_deref()
    }
}

#[derive(FromForm, Debug)]
pub struct BoardForm<'r> {
    pub name: NonEmptyStr<'r>,
    pub title: NonEmptyStr<'r>,
}

#[derive(Debug)]
pub struct Bytes(Vec<u8>);

impl Deref for Bytes {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

#[async_trait]
impl<'v> FromFormField<'v> for Bytes {
    async fn from_data(field: rocket::form::DataField<'v, '_>) -> rocket::form::Result<'v, Self> {
        let stream = field.data.open(10.mebibytes());
        let buf = stream
            .into_bytes()
            .await
            .map(|v| v.into_inner())
            .map_err(|e| rocket::form::Errors::from(rocket::form::Error::custom(e)))?;
        if buf.is_empty() {
            Err(rocket::form::Error::validation("Empty files are not allowed").into())
        } else {
            Ok(Self(buf))
        }
    }

    fn from_value(field: rocket::form::ValueField<'v>) -> rocket::form::Result<'v, Self> {
        Ok(Self(field.value.as_bytes().to_owned()))
    }

    fn default() -> Option<Self> {
        None
    }
}

#[derive(Debug)]
pub struct NotBanned;

#[async_trait]
impl<'r> FromRequest<'r> for NotBanned {
    type Error = Error;

    async fn from_request(request: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        let pool = request.rocket().state::<PgPool>().unwrap();
        let ip: IpNetwork = request.client_ip().unwrap().into();
        let ban = match query!(
            "SELECT reason
            FROM bans
            WHERE $1 <<= ip AND created_at + duration > NOW()
            ORDER BY created_at DESC",
            ip
        )
        .fetch_optional(pool)
        .await
        {
            Ok(r) => r,
            Err(e) => {
                return request::Outcome::Failure((Status::InternalServerError, Error::Db(e)))
            }
        };

        if let Some(ban) = ban {
            request::Outcome::Failure((Status::Forbidden, Error::Banned(ban.reason)))
        } else {
            request::Outcome::Success(Self)
        }
    }
}

pub struct Captcha {
    id: Uuid,
    base64image: String,
    solution: String,
}

impl Captcha {
    pub async fn new(pool: &PgPool) -> Result<Self, Error> {
        let (id, base64image, solution) = {
            let mut captcha = captcha::RngCaptcha::<StdRng>::new();
            captcha.add_chars(6);

            let mut geom = captcha.text_area();
            geom.left -= 10;
            geom.right += 10;
            geom.top -= 10;
            geom.bottom += 10;
            let captcha = captcha.extract(geom);
            captcha
                .apply_filter(captcha::filters::Wave::new(10.0, 2.0).horizontal())
                .apply_filter(captcha::filters::Grid::new(8, 8))
                .apply_filter(captcha::filters::Wave::new(10.0, 2.0).vertical());

            (
                Uuid::from_bytes(*uuid::Uuid::new_v4().as_bytes()),
                captcha.as_base64().unwrap(),
                captcha.chars_as_string().to_lowercase(),
            )
        };

        query!(
            "INSERT INTO captchas(id, solution) VALUES ($1, $2)",
            id,
            solution
        )
        .execute(pool)
        .await?;

        Ok(Self {
            id,
            base64image,
            solution,
        })
    }

    pub async fn verify(id: Uuid, answer: &str, pool: &PgPool) -> Result<bool, Error> {
        let captcha = query!(
            "DELETE FROM captchas
            WHERE id = $1
            RETURNING solution",
            id
        )
        .fetch_optional(pool)
        .await?;

        if let Some(captcha) = captcha {
            Ok(captcha.solution == answer.to_lowercase())
        } else {
            Ok(false)
        }
    }

    pub fn base64image(&self) -> &str {
        self.base64image.as_ref()
    }

    pub fn solution(&self) -> &str {
        self.solution.as_ref()
    }

    pub fn id(&self) -> Uuid {
        self.id
    }
}

#[derive(sqlx::Type)]
#[sqlx(type_name = "privelege_level")]
#[sqlx(rename_all = "lowercase")]
pub enum PrivelegeLevel {
    Admin,
    Mod,
}

pub struct User {
    id: Uuid,
    name: String,
    level: PrivelegeLevel,
}

impl User {
    pub async fn new(name: &str, level: PrivelegeLevel, pool: &PgPool) -> Result<Self, Error> {
        let id = Uuid::from_bytes(uuid::Uuid::new_v4().into_bytes());
        let user = query_as!(
            User,
            r#"INSERT INTO users(id, name, level)
            VALUES ($1, $2, $3)
            RETURNING id, name, level AS "level!: PrivelegeLevel""#,
            id,
            name,
            level as PrivelegeLevel
        )
        .fetch_one(pool)
        .await?;
        Ok(user)
    }
}

pub struct Session {
    id: Uuid,
    uid: Uuid,
    logged_in_at: PrimitiveDateTime,
}

impl Session {
    pub async fn get(id: Uuid, pool: &PgPool) -> Result<Option<Self>, Error> {
        let session = query_as!(Session, "SELECT * FROM sessions WHERE id = $1", id)
            .fetch_optional(pool)
            .await?;
        Ok(session)
    }

    pub async fn new(name: &str, password: &str, pool: &PgPool) -> Result<Self, Error> {
        let id = Uuid::from_bytes(uuid::Uuid::new_v4().into_bytes());
        let uid: Uuid = todo!();
        let session = query_as!(
            Session,
            "INSERT INTO sessions (id, uid) VALUES ($1, $2) RETURNING *",
            id,
            uid
        )
        .fetch_one(pool)
        .await?;
        Ok(session)
    }

    pub fn uid(&self) -> Uuid {
        self.uid
    }
}

pub struct AdminPrivilege {
    uid: Uuid,
}

impl AdminPrivilege {
    pub fn uid(&self) -> Uuid {
        self.uid
    }
}

#[async_trait]
impl<'r> FromRequest<'r> for AdminPrivilege {
    type Error = Error;

    async fn from_request(request: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        let pool = request.rocket().state::<PgPool>().unwrap();
        let session = request.cookies().get_private("sessionid");
        let session = session.map(|c| c.value().parse());
        if let Some(Ok(session)) = session {
            if let Ok(Some(session)) = Session::get(session, pool).await {
                return request::Outcome::Success(Self { uid: session.uid() });
            }
        }
        request::Outcome::Forward(())
    }
}

#[derive(FromForm)]
pub struct LoginForm<'r> {
    name: NonEmptyStr<'r>,
    password: NonEmptyStr<'r>,
}
