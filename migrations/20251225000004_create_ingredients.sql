-- Create ingredients table
CREATE TABLE ingredients (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    recipe_id UUID NOT NULL REFERENCES recipes(id) ON DELETE CASCADE,
    position INTEGER NOT NULL CHECK (position >= 1 AND position <= 50),
    quantity DECIMAL(10, 3),
    unit VARCHAR(20),
    name VARCHAR(200) NOT NULL,
    notes VARCHAR(200),

    CONSTRAINT ingredients_recipe_position_unique UNIQUE (recipe_id, position)
);

-- Indexes
CREATE INDEX idx_ingredients_recipe_id ON ingredients(recipe_id);
