-- Create book_contributions table for collaborative book editing
-- Tracks which recipes were added to a book by contributors
-- Recipe ownership remains with the original author

CREATE TABLE book_contributions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    book_id UUID NOT NULL REFERENCES recipe_books(id) ON DELETE CASCADE,
    recipe_id UUID NOT NULL REFERENCES recipes(id) ON DELETE CASCADE,
    contributor_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    added_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- A recipe can only be added once per book
    UNIQUE (book_id, recipe_id)
);

-- Index for finding all contributions to a book
CREATE INDEX idx_book_contributions_book ON book_contributions(book_id);

-- Index for finding all contributions by a user
CREATE INDEX idx_book_contributions_contributor ON book_contributions(contributor_id);
