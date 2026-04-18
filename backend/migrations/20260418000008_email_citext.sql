-- Email comparisons must be case-insensitive: USER@example.com and user@example.com
-- are the same address. Using CITEXT delegates this to the DB, eliminating the class
-- of login/unlock bugs caused by any lookup forgetting to lowercase the input.
ALTER TABLE users ALTER COLUMN email TYPE CITEXT;
