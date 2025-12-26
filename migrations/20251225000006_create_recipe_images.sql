-- Create recipe_images table
CREATE TABLE recipe_images (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    recipe_id UUID NOT NULL REFERENCES recipes(id) ON DELETE CASCADE,
    url VARCHAR(2048) NOT NULL,
    alt_text VARCHAR(200),
    position INTEGER NOT NULL CHECK (position >= 1 AND position <= 10),
    is_primary BOOLEAN NOT NULL DEFAULT false,

    CONSTRAINT recipe_images_recipe_position_unique UNIQUE (recipe_id, position)
);

-- Indexes
CREATE INDEX idx_recipe_images_recipe_id ON recipe_images(recipe_id);

-- Partial unique index to ensure only one primary image per recipe
CREATE UNIQUE INDEX idx_recipe_images_primary ON recipe_images(recipe_id) WHERE is_primary = true;
