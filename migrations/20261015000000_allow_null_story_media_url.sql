-- migrations/20261015000000_allow_null_story_media_url.sql
-- Allow NULL for stories.media_url to support text-only stories

ALTER TABLE stories ALTER COLUMN media_url DROP NOT NULL;
