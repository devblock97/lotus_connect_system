-- migrations/20260918000000_add_avatar_url_to_users.sql
ALTER TABLE users ADD COLUMN IF NOT EXISTS avatar_url TEXT;
