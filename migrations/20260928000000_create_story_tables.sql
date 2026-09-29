-- migrations/20260928000000_create_story_tables.sql

-- 1. STORIES TABLE
CREATE TABLE IF NOT EXISTS stories (
    id UUID PRIMARY KEY,
    author_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    media_type VARCHAR(20) NOT NULL DEFAULT 'image', -- 'image', 'video', 'text'
    media_url TEXT NOT NULL,
    thumbnail_url TEXT,
    caption TEXT,
    duration DOUBLE PRECISION NOT NULL DEFAULT 5.0,
    visibility VARCHAR(20) NOT NULL DEFAULT 'friends', -- 'public', 'friends', 'close_friends'
    background_color VARCHAR(30),
    metadata JSONB DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL DEFAULT (NOW() + INTERVAL '24 hours')
);

CREATE INDEX IF NOT EXISTS idx_stories_author_created ON stories(author_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_stories_expires_at ON stories(expires_at DESC);
CREATE INDEX IF NOT EXISTS idx_stories_visibility ON stories(visibility);

-- 2. STORY VIEWS TABLE (Instagram Seen / Viewers list)
CREATE TABLE IF NOT EXISTS story_views (
    id UUID PRIMARY KEY,
    story_id UUID NOT NULL REFERENCES stories(id) ON DELETE CASCADE,
    viewer_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    viewed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT unique_story_viewer UNIQUE(story_id, viewer_id)
);

CREATE INDEX IF NOT EXISTS idx_story_views_story ON story_views(story_id, viewed_at DESC);
CREATE INDEX IF NOT EXISTS idx_story_views_viewer ON story_views(viewer_id);

-- 3. STORY REACTIONS TABLE (Instagram Quick Reactions)
CREATE TABLE IF NOT EXISTS story_reactions (
    id UUID PRIMARY KEY,
    story_id UUID NOT NULL REFERENCES stories(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    reaction VARCHAR(32) NOT NULL DEFAULT '❤️',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT unique_story_reaction UNIQUE(story_id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_story_reactions_story ON story_reactions(story_id);
CREATE INDEX IF NOT EXISTS idx_story_reactions_user ON story_reactions(user_id);

-- 4. CLOSE FRIENDS TABLE (Instagram Close Friends - Green ring)
CREATE TABLE IF NOT EXISTS close_friends (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    friend_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT unique_user_close_friend UNIQUE(user_id, friend_id)
);

CREATE INDEX IF NOT EXISTS idx_close_friends_user ON close_friends(user_id);
CREATE INDEX IF NOT EXISTS idx_close_friends_friend ON close_friends(friend_id);
