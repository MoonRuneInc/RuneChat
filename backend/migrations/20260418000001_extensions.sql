-- citext provides native case-insensitive text type used for username uniqueness
CREATE EXTENSION IF NOT EXISTS "citext";

-- pgcrypto provides gen_random_uuid() used across all tables
CREATE EXTENSION IF NOT EXISTS "pgcrypto";
