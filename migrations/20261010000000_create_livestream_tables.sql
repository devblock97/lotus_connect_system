-- migrations/20261010000000_create_livestream_tables.sql

-- LIVE STREAMS TABLE
CREATE TABLE IF NOT EXISTS live_streams (
    id UUID PRIMARY KEY,
    host_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title VARCHAR(255) NOT NULL,
    description TEXT,
    thumbnail_url TEXT,
    status VARCHAR(32) NOT NULL DEFAULT 'created', -- 'created', 'live', 'ended'
    privacy VARCHAR(32) NOT NULL DEFAULT 'public', -- 'public', 'friends', 'private'
    stream_key VARCHAR(128) NOT NULL UNIQUE,
    ingest_url TEXT,
    playback_url TEXT,
    channel_id VARCHAR(128),
    viewer_count INT NOT NULL DEFAULT 0,
    peak_viewers INT NOT NULL DEFAULT 0,
    total_views INT NOT NULL DEFAULT 0,
    like_count BIGINT NOT NULL DEFAULT 0,
    pinned_comment_id UUID,
    started_at TIMESTAMPTZ,
    ended_at TIMESTAMPTZ,
    duration_seconds INT NOT NULL DEFAULT 0,
    recording_url TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_live_streams_host ON live_streams(host_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_live_streams_status ON live_streams(status, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_live_streams_stream_key ON live_streams(stream_key);

-- LIVE STREAM VIEWERS TABLE (Room membership & active viewer tracking)
CREATE TABLE IF NOT EXISTS live_stream_viewers (
    id UUID PRIMARY KEY,
    stream_id UUID NOT NULL REFERENCES live_streams(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    joined_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    left_at TIMESTAMPTZ,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    CONSTRAINT unique_live_stream_viewer UNIQUE(stream_id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_live_stream_viewers_stream ON live_stream_viewers(stream_id, is_active);
CREATE INDEX IF NOT EXISTS idx_live_stream_viewers_user ON live_stream_viewers(user_id);

-- LIVE STREAM COMMENTS TABLE (Real-time live chat)
CREATE TABLE IF NOT EXISTS live_stream_comments (
    id UUID PRIMARY KEY,
    stream_id UUID NOT NULL REFERENCES live_streams(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    content TEXT NOT NULL,
    is_pinned BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_live_stream_comments_stream ON live_stream_comments(stream_id, created_at ASC);

-- LIVE STREAM GIFTS TABLE
CREATE TABLE IF NOT EXISTS live_stream_gifts (
    id UUID PRIMARY KEY,
    stream_id UUID NOT NULL REFERENCES live_streams(id) ON DELETE CASCADE,
    sender_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    gift_type VARCHAR(64) NOT NULL, -- 'rose', 'heart', 'diamond', 'coffee', 'firework', 'rocket', 'crown'
    gift_count INT NOT NULL DEFAULT 1,
    coin_amount INT NOT NULL DEFAULT 1,
    combo_count INT NOT NULL DEFAULT 1,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_live_stream_gifts_stream ON live_stream_gifts(stream_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_live_stream_gifts_sender ON live_stream_gifts(stream_id, sender_id);

-- LIVE STREAM CO-HOSTS TABLE
CREATE TABLE IF NOT EXISTS live_stream_cohosts (
    id UUID PRIMARY KEY,
    stream_id UUID NOT NULL REFERENCES live_streams(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    status VARCHAR(32) NOT NULL DEFAULT 'pending', -- 'pending', 'accepted', 'rejected', 'ended'
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT unique_live_stream_cohost UNIQUE(stream_id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_live_stream_cohosts_stream ON live_stream_cohosts(stream_id, status);
