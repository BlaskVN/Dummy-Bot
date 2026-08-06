CREATE TABLE donation_config (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    message TEXT,
    url TEXT,
    qr_filename TEXT,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
