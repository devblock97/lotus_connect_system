-- Add media_items JSONB column to messages table for multi-media support
ALTER TABLE messages ADD COLUMN IF NOT EXISTS media_items JSONB DEFAULT '[]'::jsonb;
