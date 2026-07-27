-- migrations/20260727000000_add_full_name_to_users.sql
ALTER TABLE users ADD COLUMN IF NOT EXISTS full_name VARCHAR(255);
