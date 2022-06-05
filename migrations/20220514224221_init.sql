CREATE TABLE IF NOT EXISTS boards (
    name VARCHAR(255) UNIQUE NOT NULL PRIMARY KEY,
    title TEXT NOT NULL,
    next_post_id INTEGER NOT NULL DEFAULT 0
);
INSERT INTO boards(name, title) VALUES ('b', 'Random');

CREATE TABLE IF NOT EXISTS posts (
    id INTEGER NOT NULL,
    board VARCHAR(255) REFERENCES boards(name) NOT NULL,
    PRIMARY KEY (id, board),
    title VARCHAR(255),
    author VARCHAR(255),
    email VARCHAR(255),
    sage BOOLEAN NOT NULL,
    plaintext_content VARCHAR(65535),
    html_content VARCHAR(65535) NOT NULL,
    thread INTEGER NOT NULL,
    FOREIGN KEY (thread, board) REFERENCES posts(id, board),
    posted_at TIMESTAMP NOT NULL DEFAULT NOW()
);
