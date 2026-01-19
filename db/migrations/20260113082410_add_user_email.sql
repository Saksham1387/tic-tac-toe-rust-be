-- Add migration script here
ALTER TABLE users
ADD COLUMN email VARCHAR(255) UNIQUE NOT NULL;

CREATE INDEX idx_users_email ON users(email);