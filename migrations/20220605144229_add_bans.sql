CREATE TABLE IF NOT EXISTS bans (
    ip INET NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    duration INTERVAL NOT NULL,
    PRIMARY KEY (ip, created_at, duration),
    reason VARCHAR(256) NOT NULL
);

ALTER TABLE IF EXISTS posts
    ADD COLUMN IF NOT EXISTS
        ip INET NOT NULL;
