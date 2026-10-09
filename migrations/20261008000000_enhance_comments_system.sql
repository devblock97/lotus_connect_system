-- migrations/20261008000000_enhance_comments_system.sql

-- Enhance post_comments table with rich features
ALTER TABLE post_comments
    ADD COLUMN IF NOT EXISTS media_url TEXT,
    ADD COLUMN IF NOT EXISTS like_count BIGINT NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS reply_count BIGINT NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS is_pinned BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN IF NOT EXISTS pinned_at TIMESTAMPTZ;

-- Create comment_reactions table for Facebook / Instagram / TikTok style reactions
CREATE TABLE IF NOT EXISTS comment_reactions (
    id UUID PRIMARY KEY,
    comment_id UUID NOT NULL REFERENCES post_comments(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    reaction VARCHAR(32) NOT NULL DEFAULT 'like',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT unique_user_comment_reaction UNIQUE(comment_id, user_id)
);

-- Indexes for fast retrieval, sorting, pagination, and reaction lookup
CREATE INDEX IF NOT EXISTS idx_comment_reactions_comment ON comment_reactions(comment_id);
CREATE INDEX IF NOT EXISTS idx_comment_reactions_user ON comment_reactions(user_id);
CREATE INDEX IF NOT EXISTS idx_post_comments_post_parent ON post_comments(post_id, parent_comment_id);
CREATE INDEX IF NOT EXISTS idx_post_comments_sorting ON post_comments(post_id, is_pinned DESC, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_post_comments_popular ON post_comments(post_id, is_pinned DESC, like_count DESC, created_at DESC);

-- Backfill reply_count for any existing comments
UPDATE post_comments p
SET reply_count = COALESCE((
    SELECT COUNT(*) FROM post_comments c WHERE c.parent_comment_id = p.id
), 0);
