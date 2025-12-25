-- Enum types for Oppskrift
-- Migration: 20251225000001_create_enums

-- Visibility for recipes and recipe books
CREATE TYPE visibility_type AS ENUM ('public', 'private');

-- Recipe difficulty levels
CREATE TYPE difficulty_type AS ENUM ('easy', 'medium', 'hard');

-- User measurement preference
CREATE TYPE measurement_pref AS ENUM ('metric', 'imperial');

-- Activity types for the activity feed
CREATE TYPE activity_type AS ENUM ('create', 'share', 'follow');

-- Target types for polymorphic activity references
CREATE TYPE target_type AS ENUM ('recipe', 'book', 'user');
