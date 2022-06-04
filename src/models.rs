use rocket::FromForm;
use sqlx::{query, query_as, types::time::PrimitiveDateTime, PgPool};
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

    pub async fn create(name: String, title: String, pool: &PgPool) -> Result<Board, sqlx::Error> {
        query!(
            "INSERT INTO boards(name, title)
                VALUES ($1, $2)",
            name,
            title
        )
        .execute(pool)
        .await?;
        Ok(Board { name, title })
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
    content: Option<String>,
    posted_at: PrimitiveDateTime,
    thread: i32,
}

impl Post {
    pub async fn for_thread(board: &str, id: i32, pool: &PgPool) -> Result<Vec<Post>, sqlx::Error> {
        query_as!(
            Post,
            "SELECT * FROM posts WHERE thread = $1 AND board = $2",
            id,
            board
        )
        .fetch_all(pool)
        .await
    }

    pub async fn threads_for_board(board: &str, pool: &PgPool) -> Result<Vec<Post>, sqlx::Error> {
        query_as!(
            Post,
            "WITH threads AS (
                SELECT posts.thread as id, max(posts.posted_at) as last_post
                FROM posts
                WHERE posts.board = $1
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

    pub async fn create_thread(
        board: &str,
        title: Option<&str>,
        author: Option<&str>,
        email: Option<&str>,
        sage: bool,
        content: Option<&str>,
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

        let id = query!(
            "INSERT INTO posts(id, board, title, author, email, sage, content, thread)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $1)
            RETURNING id;",
            per_board_id,
            board,
            title,
            author,
            email,
            sage,
            content
        )
        .fetch_one(&mut tx)
        .await
        .map(|r| r.id);

        tx.commit().await?;

        id
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

        let id = query!(
            "INSERT INTO posts(id, board, title, author, email, sage, content, thread)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING id;",
            per_board_id,
            board,
            title,
            author,
            email,
            sage,
            content,
            thread
        )
        .fetch_one(pool)
        .await
        .map(|r| r.id);

        tx.commit().await?;
        id
    }

    /// Get the post's id.
    #[must_use]
    pub fn id(&self) -> i32 {
        self.id
    }

    /// Get a reference to the post's content.
    #[must_use]
    pub fn content(&self) -> Option<&str> {
        self.content.as_deref()
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
}

#[derive(FromForm, Debug)]
pub struct PostForm {
    pub title: Option<String>,
    pub author: Option<String>,
    pub email: Option<String>,
    pub sage: bool,
    pub content: Option<String>,
    pub thread: Option<i32>,
    pub board: String,
}

#[derive(FromForm, Debug)]
pub struct BoardForm {
    pub name: String,
    pub title: String,
}
