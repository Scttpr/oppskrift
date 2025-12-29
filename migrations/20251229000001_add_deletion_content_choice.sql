-- Add deletion content choice enum
-- Migration: 20251229000001_add_deletion_content_choice
-- Feature: 004-user-profile-settings

-- Enum for what happens to user's content on account deletion
CREATE TYPE deletion_content_choice AS ENUM (
    'anonymize',  -- Keep recipes/books with "Deleted User" attribution
    'delete_all'  -- Remove all user's content (recipes, books, comments)
);

-- Add column to users table
ALTER TABLE users ADD COLUMN deletion_content_choice deletion_content_choice;

COMMENT ON COLUMN users.deletion_content_choice IS 'User choice for content handling on account deletion. NULL until deletion is requested.';
