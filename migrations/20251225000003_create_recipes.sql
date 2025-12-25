-- Create recipes table
CREATE TABLE recipes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    author_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title VARCHAR(200) NOT NULL,
    description TEXT,
    visibility visibility_type NOT NULL DEFAULT 'public',
    prep_time_min INTEGER CHECK (prep_time_min >= 0),
    cook_time_min INTEGER CHECK (cook_time_min >= 0),
    servings VARCHAR(50),
    difficulty difficulty_type,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ap_id VARCHAR(2048) NOT NULL UNIQUE,
    search_vector TSVECTOR
);

-- Indexes
CREATE INDEX idx_recipes_author_id ON recipes(author_id);
CREATE INDEX idx_recipes_visibility ON recipes(visibility);
CREATE INDEX idx_recipes_created_at ON recipes(created_at DESC);
CREATE INDEX idx_recipes_ap_id ON recipes(ap_id);
CREATE INDEX idx_recipes_search ON recipes USING GIN(search_vector);

-- Composite index for public recipe listing
CREATE INDEX idx_recipes_public_listing ON recipes(visibility, created_at DESC)
    INCLUDE (id, title, author_id)
    WHERE visibility = 'public';

-- Auto-update updated_at trigger
CREATE TRIGGER set_recipes_updated_at
    BEFORE UPDATE ON recipes
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

-- Full-text search trigger
CREATE OR REPLACE FUNCTION recipe_search_update()
RETURNS TRIGGER AS $$
BEGIN
    NEW.search_vector :=
        setweight(to_tsvector('english', COALESCE(NEW.title, '')), 'A') ||
        setweight(to_tsvector('english', COALESCE(NEW.description, '')), 'B');
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER recipe_search_trigger
    BEFORE INSERT OR UPDATE ON recipes
    FOR EACH ROW
    EXECUTE FUNCTION recipe_search_update();
