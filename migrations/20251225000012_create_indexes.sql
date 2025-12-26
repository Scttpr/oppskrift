-- Database indexes for performance optimization

-- User indexes
CREATE INDEX IF NOT EXISTS idx_users_username ON users(username);
CREATE INDEX IF NOT EXISTS idx_users_ap_id ON users(ap_id);

-- Recipe indexes
CREATE INDEX IF NOT EXISTS idx_recipes_author_id ON recipes(author_id);
CREATE INDEX IF NOT EXISTS idx_recipes_visibility ON recipes(visibility);
CREATE INDEX IF NOT EXISTS idx_recipes_created_at ON recipes(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_recipes_ap_id ON recipes(ap_id);

-- Ingredient indexes
CREATE INDEX IF NOT EXISTS idx_ingredients_recipe_id ON ingredients(recipe_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_ingredients_recipe_position ON ingredients(recipe_id, position);

-- Instruction step indexes
CREATE INDEX IF NOT EXISTS idx_instruction_steps_recipe_id ON instruction_steps(recipe_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_instruction_steps_recipe_step ON instruction_steps(recipe_id, step_number);

-- Recipe image indexes
CREATE INDEX IF NOT EXISTS idx_recipe_images_recipe_id ON recipe_images(recipe_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_recipe_images_position ON recipe_images(recipe_id, position);
CREATE UNIQUE INDEX IF NOT EXISTS idx_recipe_images_primary ON recipe_images(recipe_id) WHERE is_primary = true;

-- Recipe book indexes
CREATE INDEX IF NOT EXISTS idx_recipe_books_owner_id ON recipe_books(owner_id);
CREATE INDEX IF NOT EXISTS idx_recipe_books_visibility ON recipe_books(visibility);
CREATE INDEX IF NOT EXISTS idx_recipe_books_ap_id ON recipe_books(ap_id);

-- Book recipe entry indexes
CREATE INDEX IF NOT EXISTS idx_book_recipe_entries_book_id ON book_recipe_entries(book_id);
CREATE INDEX IF NOT EXISTS idx_book_recipe_entries_recipe_id ON book_recipe_entries(recipe_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_book_recipe_entries_unique ON book_recipe_entries(book_id, recipe_id);
CREATE INDEX IF NOT EXISTS idx_book_recipe_entries_position ON book_recipe_entries(book_id, position);

-- Saved recipe indexes
CREATE INDEX IF NOT EXISTS idx_saved_recipes_user_id ON saved_recipes(user_id);
CREATE INDEX IF NOT EXISTS idx_saved_recipes_recipe_id ON saved_recipes(recipe_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_saved_recipes_unique ON saved_recipes(user_id, recipe_id);

-- Follow indexes
CREATE INDEX IF NOT EXISTS idx_follows_follower_id ON follows(follower_id);
CREATE INDEX IF NOT EXISTS idx_follows_following_id ON follows(following_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_follows_unique ON follows(follower_id, following_id);

-- Activity indexes
CREATE INDEX IF NOT EXISTS idx_activities_actor_id ON activities(actor_id);
CREATE INDEX IF NOT EXISTS idx_activities_created_at ON activities(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_activities_target ON activities(target_type, target_id);
CREATE INDEX IF NOT EXISTS idx_activities_actor_created ON activities(actor_id, created_at DESC);
