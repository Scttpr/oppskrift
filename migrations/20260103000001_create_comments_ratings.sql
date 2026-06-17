-- Comments on recipes
CREATE TABLE recipe_comments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    recipe_id UUID NOT NULL REFERENCES recipes(id) ON DELETE CASCADE,
    author_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    body TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_recipe_comments_recipe_id ON recipe_comments(recipe_id, created_at DESC);
CREATE INDEX idx_recipe_comments_author_id ON recipe_comments(author_id);

CREATE TRIGGER set_recipe_comments_updated_at
    BEFORE UPDATE ON recipe_comments
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

-- Star ratings on recipes (one per user per recipe)
CREATE TABLE recipe_ratings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    recipe_id UUID NOT NULL REFERENCES recipes(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    value SMALLINT NOT NULL CHECK (value BETWEEN 1 AND 5),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (recipe_id, user_id)
);

CREATE INDEX idx_recipe_ratings_recipe_id ON recipe_ratings(recipe_id);

CREATE TRIGGER set_recipe_ratings_updated_at
    BEFORE UPDATE ON recipe_ratings
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();
