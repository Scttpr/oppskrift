-- Create book_recipe_entries table (junction table for recipes in books)
CREATE TABLE book_recipe_entries (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    book_id UUID NOT NULL REFERENCES recipe_books(id) ON DELETE CASCADE,
    recipe_id UUID NOT NULL REFERENCES recipes(id) ON DELETE CASCADE,
    position INTEGER NOT NULL,
    added_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT book_recipe_entries_unique UNIQUE (book_id, recipe_id)
);

-- Indexes
CREATE INDEX idx_book_recipe_entries_book_id ON book_recipe_entries(book_id);
CREATE INDEX idx_book_recipe_entries_recipe_id ON book_recipe_entries(recipe_id);
CREATE INDEX idx_book_recipe_entries_position ON book_recipe_entries(book_id, position);
