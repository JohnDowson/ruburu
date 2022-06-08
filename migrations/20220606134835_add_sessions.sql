CREATE TYPE privelege_level AS ENUM (
    'admin',
    'mod'
);

CREATE TABLE IF NOT EXISTS users (
    id UUID NOT NULL UNIQUE,
    name VARCHAR(255) NOT NULL UNIQUE,
    PRIMARY KEY (id, name),
    password BYTEA NOT NULL,
    level privelege_level NOT NULL
);

CREATE TABLE IF NOT EXISTS sessions (
    id UUID PRIMARY KEY NOT NULL,
    uid UUID REFERENCES users(id) NOT NULL,
    logged_in_at TIMESTAMP NOT NULL DEFAULT NOW()
);
