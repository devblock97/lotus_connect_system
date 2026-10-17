-- migrations/20261017000000_add_notifications_pagination_index.sql

-- Composite index for efficient cursor-based pagination on notifications
CREATE INDEX IF NOT EXISTS idx_notifications_user_created ON notifications(user_id, created_at DESC, id DESC);
