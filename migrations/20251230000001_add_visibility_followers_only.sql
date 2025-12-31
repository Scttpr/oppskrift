-- Add followers_only visibility type
-- This allows content to be visible only to the owner's followers

ALTER TYPE visibility_type ADD VALUE IF NOT EXISTS 'followers_only';
