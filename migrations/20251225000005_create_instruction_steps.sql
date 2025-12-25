-- Create instruction_steps table
CREATE TABLE instruction_steps (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    recipe_id UUID NOT NULL REFERENCES recipes(id) ON DELETE CASCADE,
    step_number INTEGER NOT NULL CHECK (step_number >= 1 AND step_number <= 30),
    description TEXT NOT NULL,
    image_url VARCHAR(2048),
    duration_min INTEGER CHECK (duration_min >= 0),

    CONSTRAINT instruction_steps_recipe_step_unique UNIQUE (recipe_id, step_number)
);

-- Indexes
CREATE INDEX idx_instruction_steps_recipe_id ON instruction_steps(recipe_id);
