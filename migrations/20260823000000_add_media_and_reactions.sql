-- Add media fields to messages table
ALTER TABLE messages ADD COLUMN IF NOT EXISTS media_url TEXT;
ALTER TABLE messages ADD COLUMN IF NOT EXISTS thumbnail_url TEXT;
ALTER TABLE messages ADD COLUMN IF NOT EXISTS file_name TEXT;
ALTER TABLE messages ADD COLUMN IF NOT EXISTS file_size BIGINT;
ALTER TABLE messages ADD COLUMN IF NOT EXISTS mime_type VARCHAR(100);
ALTER TABLE messages ADD COLUMN IF NOT EXISTS duration INT;

-- Create message_reactions table
CREATE TABLE IF NOT EXISTS message_reactions (
    id UUID PRIMARY KEY,
    message_id UUID NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    reaction VARCHAR(32) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT unique_user_message_reaction UNIQUE(message_id, user_id, reaction)
);

CREATE INDEX IF NOT EXISTS idx_message_reactions_message ON message_reactions(message_id);
