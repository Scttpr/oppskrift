-- Create recipe_books table
CREATE TABLE recipe_books (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    owner_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title VARCHAR(200) NOT NULL,
    description TEXT,
    cover_image_url VARCHAR(2048),
    visibility visibility_type NOT NULL DEFAULT 'public',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ap_id VARCHAR(2048) NOT NULL UNIQUE
);

-- Indexes
CREATE INDEX idx_recipe_books_owner_id ON recipe_books(owner_id);
CREATE INDEX idx_recipe_books_visibility ON recipe_books(visibility);
CREATE INDEX idx_recipe_books_ap_id ON recipe_books(ap_id);

-- Auto-update updated_at trigger
CREATE TRIGGER set_recipe_books_updated_at
    BEFORE UPDATE ON recipe_books
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();
