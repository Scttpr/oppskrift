-- Add status and rejection_reason columns to book_contributions
-- This enables the contribution workflow: pending -> accepted/rejected

ALTER TABLE book_contributions
ADD COLUMN status VARCHAR(20) NOT NULL DEFAULT 'accepted';

ALTER TABLE book_contributions
ADD COLUMN rejection_reason TEXT;

-- Add constraint for valid statuses
ALTER TABLE book_contributions
ADD CONSTRAINT chk_contribution_status
CHECK (status IN ('pending', 'accepted', 'rejected'));

-- Add index for querying by status
CREATE INDEX idx_book_contributions_status ON book_contributions(status);

-- Add index for querying pending contributions by book
CREATE INDEX idx_book_contributions_book_status ON book_contributions(book_id, status);
