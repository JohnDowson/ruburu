CREATE TABLE IF NOT EXISTS replies (
    message_id INTEGER NOT NULL,
    message_board VARCHAR(255) REFERENCES boards(name) NOT NULL,
    FOREIGN KEY (message_id, message_board) REFERENCES posts(id, board),
    reply_id INTEGER NOT NULL,
    reply_board VARCHAR(255) REFERENCES boards(name) NOT NULL,
    reply_thread INTEGER NOT NULL,
    FOREIGN KEY (reply_id, reply_board) REFERENCES posts(id, board),
    FOREIGN KEY (reply_thread, reply_board) REFERENCES posts(id, board),
    PRIMARY KEY (message_id, message_board, reply_id, reply_board, reply_thread)
);
