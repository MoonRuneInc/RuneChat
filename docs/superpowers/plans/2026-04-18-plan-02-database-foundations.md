# Database Foundations Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Create all SQLx migrations for the RuneChat schema — every table, index, and constraint — so the database is fully defined before any feature code is written.

**Architecture:** SQLx compile-time checked migrations in `backend/migrations/`. Each migration is a single `.sql` file. Migrations run automatically on app startup (`sqlx::migrate!()`). Tests run against a real Postgres instance (Docker).

**Tech Stack:** PostgreSQL 16, SQLx 0.7, `sqlx-cli`

---

## File Map

**Create:**
- `backend/migrations/20260418000001_extensions.sql`
- `backend/migrations/20260418000002_users.sql`
- `backend/migrations/20260418000003_servers.sql`
- `backend/migrations/20260418000004_channels.sql`
- `backend/migrations/20260418000005_messages.sql`
- `backend/migrations/20260418000006_invites.sql`
- `backend/migrations/20260418000007_indexes.sql`
- `backend/src/db/mod.rs` — db module root
- `backend/src/db/migrate.rs` — migration test helper

**Modify:**
- `backend/src/main.rs` — already calls `sqlx::migrate!()`, no change needed
- `backend/Cargo.toml` — add `sqlx-cli` note (installed separately)

---

## Task 0: Set Git Identity

- [ ] **Step 1: Configure git identity**

```bash
git config user.name "Maya Kade"
git config user.email "maya@moonrune.cc"
```

- [ ] **Step 2: Verify**

```bash
git config user.name && git config user.email
```

Expected: `Maya Kade` / `maya@moonrune.cc`

---

## Task 1: Install sqlx-cli

- [ ] **Step 1: Install sqlx-cli**

```bash
cargo install sqlx-cli --no-default-features --features rustls,postgres
```

- [ ] **Step 2: Start test Postgres**

```bash
docker compose up db -d
sleep 5  # wait for healthcheck
```

- [ ] **Step 3: Export DATABASE_URL**

```bash
export DATABASE_URL="postgres://runechat:runechat@localhost:5432/runechat"
```

- [ ] **Step 4: Create database**

```bash
cd backend && sqlx database create
```

Expected: No errors. Database `runechat` created.

---

## Task 2: Extensions Migration

**Files:**
- Create: `backend/migrations/20260418000001_extensions.sql`

- [ ] **Step 1: Write migration**

```sql
-- citext provides native case-insensitive text type used for username uniqueness
CREATE EXTENSION IF NOT EXISTS "citext";

-- pgcrypto provides gen_random_uuid() used across all tables
CREATE EXTENSION IF NOT EXISTS "pgcrypto";
```

- [ ] **Step 2: Run migration**

```bash
cd backend && sqlx migrate run
```

Expected: `Applied 20260418000001/migrate extensions`

- [ ] **Step 3: Commit**

```bash
git add backend/migrations/20260418000001_extensions.sql
git commit -m "feat(db): add citext and pgcrypto extensions migration"
```

---

## Task 3: Users Migration

**Files:**
- Create: `backend/migrations/20260418000002_users.sql`

- [ ] **Step 1: Write migration**

```sql
CREATE TABLE users (
    id                      UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    username                CITEXT NOT NULL UNIQUE,
    email                   TEXT NOT NULL UNIQUE,
    password_hash           TEXT NOT NULL,
    account_status          TEXT NOT NULL DEFAULT 'active'
                                CHECK (account_status IN ('active', 'compromised', 'suspended')),
    compromise_detected_at  TIMESTAMPTZ,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE totp_secrets (
    id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id          UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    secret_encrypted TEXT NOT NULL,
    enrolled_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    verified_at      TIMESTAMPTZ
);

CREATE TABLE refresh_tokens (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash  TEXT NOT NULL UNIQUE,
    expires_at  TIMESTAMPTZ NOT NULL,
    revoked_at  TIMESTAMPTZ,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Replay detection: token exists + revoked_at IS NOT NULL = replay attack
-- Cleanup query target: revoked_at IS NOT NULL AND expires_at < now() - interval '7 days'
CREATE INDEX idx_refresh_tokens_user_expiry ON refresh_tokens (user_id, expires_at);
```

- [ ] **Step 2: Run migration**

```bash
cd backend && sqlx migrate run
```

Expected: `Applied 20260418000002/migrate users`

- [ ] **Step 3: Verify schema**

```bash
docker compose exec db psql -U runechat -c "\d users"
docker compose exec db psql -U runechat -c "\d refresh_tokens"
```

Expected: Tables exist with correct columns including `CITEXT` for username and `revoked_at` on refresh_tokens.

- [ ] **Step 4: Commit**

```bash
git add backend/migrations/20260418000002_users.sql
git commit -m "feat(db): add users, totp_secrets, refresh_tokens tables"
```

---

## Task 4: Servers Migration

**Files:**
- Create: `backend/migrations/20260418000003_servers.sql`

- [ ] **Step 1: Write migration**

```sql
CREATE TABLE servers (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name       TEXT NOT NULL,
    owner_id   UUID NOT NULL REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE server_members (
    server_id UUID NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    user_id   UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role      TEXT NOT NULL DEFAULT 'member'
                  CHECK (role IN ('owner', 'admin', 'member')),
    joined_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (server_id, user_id)
);

CREATE INDEX idx_server_members_user_server ON server_members (user_id, server_id);
```

- [ ] **Step 2: Run migration**

```bash
cd backend && sqlx migrate run
```

Expected: `Applied 20260418000003/migrate servers`

- [ ] **Step 3: Commit**

```bash
git add backend/migrations/20260418000003_servers.sql
git commit -m "feat(db): add servers and server_members tables"
```

---

## Task 5: Channels Migration

**Files:**
- Create: `backend/migrations/20260418000004_channels.sql`

- [ ] **Step 1: Write migration**

```sql
CREATE TABLE channels (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    server_id    UUID NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    display_name TEXT NOT NULL,
    slug         TEXT NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (server_id, slug)
);

-- Supports listing and lookup by server
CREATE INDEX idx_channels_server_slug ON channels (server_id, slug);
```

- [ ] **Step 2: Run migration**

```bash
cd backend && sqlx migrate run
```

Expected: `Applied 20260418000004/migrate channels`

- [ ] **Step 3: Commit**

```bash
git add backend/migrations/20260418000004_channels.sql
git commit -m "feat(db): add channels table"
```

---

## Task 6: Messages Migration

**Files:**
- Create: `backend/migrations/20260418000005_messages.sql`

- [ ] **Step 1: Write migration**

```sql
CREATE TABLE messages (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    channel_id          UUID NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    author_id           UUID NOT NULL REFERENCES users(id),
    content             TEXT NOT NULL,
    compromised_at_send BOOLEAN NOT NULL DEFAULT false,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    edited_at           TIMESTAMPTZ
);

-- Primary query pattern: fetch last N messages in a channel ordered by time
CREATE INDEX idx_messages_channel_created ON messages (channel_id, created_at DESC);
```

- [ ] **Step 2: Run migration**

```bash
cd backend && sqlx migrate run
```

Expected: `Applied 20260418000005/migrate messages`

- [ ] **Step 3: Commit**

```bash
git add backend/migrations/20260418000005_messages.sql
git commit -m "feat(db): add messages table"
```

---

## Task 7: Invites Migration

**Files:**
- Create: `backend/migrations/20260418000006_invites.sql`

- [ ] **Step 1: Write migration**

```sql
CREATE TABLE invites (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    server_id  UUID NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    creator_id UUID NOT NULL REFERENCES users(id),
    code       TEXT NOT NULL UNIQUE,
    max_uses   INTEGER,
    uses       INTEGER NOT NULL DEFAULT 0,
    expires_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- UNIQUE constraint on code already creates a B-tree index — no additional index needed
```

- [ ] **Step 2: Run migration**

```bash
cd backend && sqlx migrate run
```

Expected: `Applied 20260418000006/migrate invites`

- [ ] **Step 3: Commit**

```bash
git add backend/migrations/20260418000006_invites.sql
git commit -m "feat(db): add invites table"
```

---

## Task 8: Migration Integration Test

**Files:**
- Create: `backend/src/db/mod.rs`
- Create: `backend/src/db/migrate.rs`

- [ ] **Step 1: Write db module**

Create `backend/src/db/mod.rs`:

```rust
pub mod migrate;
```

Create `backend/src/db/migrate.rs`:

```rust
#[cfg(test)]
mod tests {
    use sqlx::postgres::PgPoolOptions;

    #[sqlx::test(migrations = "./migrations")]
    async fn all_migrations_apply_cleanly(pool: sqlx::PgPool) {
        // sqlx::test automatically runs migrations and provides a fresh pool.
        // If we get here without panic, all migrations applied successfully.
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
            .fetch_one(&pool)
            .await
            .expect("users table must exist after migrations");
        assert_eq!(row.0, 0, "fresh database should have zero users");

        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM refresh_tokens")
            .fetch_one(&pool)
            .await
            .expect("refresh_tokens table must exist");
        assert_eq!(row.0, 0);

        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM messages")
            .fetch_one(&pool)
            .await
            .expect("messages table must exist");
        assert_eq!(row.0, 0);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn username_uniqueness_is_case_insensitive(pool: sqlx::PgPool) {
        sqlx::query(
            "INSERT INTO users (username, email, password_hash)
             VALUES ('Alice', 'alice@example.com', 'hash')"
        )
        .execute(&pool)
        .await
        .expect("insert first user");

        let result = sqlx::query(
            "INSERT INTO users (username, email, password_hash)
             VALUES ('alice', 'alice2@example.com', 'hash')"
        )
        .execute(&pool)
        .await;

        assert!(result.is_err(), "citext UNIQUE must reject case-insensitive duplicate username");
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn account_status_check_constraint_rejects_invalid_values(pool: sqlx::PgPool) {
        let result = sqlx::query(
            "INSERT INTO users (username, email, password_hash, account_status)
             VALUES ('bob', 'bob@example.com', 'hash', 'hacked')"
        )
        .execute(&pool)
        .await;

        assert!(result.is_err(), "CHECK constraint must reject invalid account_status");
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn server_member_role_check_constraint_rejects_invalid_values(pool: sqlx::PgPool) {
        // Insert prerequisite user and server
        let user_id: (uuid::Uuid,) = sqlx::query_as(
            "INSERT INTO users (username, email, password_hash) VALUES ('carol', 'carol@example.com', 'hash') RETURNING id"
        )
        .fetch_one(&pool).await.unwrap();

        let server_id: (uuid::Uuid,) = sqlx::query_as(
            "INSERT INTO servers (name, owner_id) VALUES ('Test Server', $1) RETURNING id"
        )
        .bind(user_id.0)
        .fetch_one(&pool).await.unwrap();

        let result = sqlx::query(
            "INSERT INTO server_members (server_id, user_id, role) VALUES ($1, $2, 'superuser')"
        )
        .bind(server_id.0)
        .bind(user_id.0)
        .execute(&pool)
        .await;

        assert!(result.is_err(), "CHECK constraint must reject invalid role");
    }
}
```

- [ ] **Step 2: Add db module to main.rs**

Add `mod db;` to `backend/src/main.rs` after the existing module declarations.

- [ ] **Step 3: Add sqlx test dependency**

Add to `backend/Cargo.toml` under `[dev-dependencies]`:

```toml
sqlx = { version = "0.7", features = ["runtime-tokio-rustls", "postgres", "uuid", "time", "migrate", "macros"] }
uuid = { version = "1", features = ["v4", "serde"] }
```

- [ ] **Step 4: Run tests against live database**

```bash
cd backend && DATABASE_URL="postgres://runechat:runechat@localhost:5432/runechat" cargo test db
```

Expected: 4 tests pass.

- [ ] **Step 5: Commit**

```bash
git add backend/src/db/ backend/src/main.rs backend/Cargo.toml
git commit -m "feat(db): add migration integration tests covering schema constraints"
```

---

## Task 9: Tear Down Test Database

- [ ] **Step 1: Stop Postgres**

```bash
docker compose down
```

---

## Self-Review

**Spec coverage:**

| Requirement | Status |
|---|---|
| citext extension for case-insensitive username | ✅ Migration 1 + 2 |
| users table with account_status CHECK | ✅ Migration 2 |
| totp_secrets with encrypted secret | ✅ Migration 2 |
| refresh_tokens with revoked_at for replay detection | ✅ Migration 2 |
| Refresh token cleanup index | ✅ Migration 2 |
| servers + server_members with role CHECK | ✅ Migration 3 |
| server_members membership lookup index | ✅ Migration 3 |
| channels with display_name + slug UNIQUE per server | ✅ Migration 4 |
| channels server+slug index | ✅ Migration 4 |
| messages with compromised_at_send flag | ✅ Migration 5 |
| messages channel+created_at index | ✅ Migration 5 |
| invites with max_uses + uses + expires_at | ✅ Migration 6 |
| Integration tests verify schema + constraints | ✅ Task 8 |

**Placeholder scan:** No TBDs. All SQL and Rust is complete.

**Type consistency:** `uuid::Uuid` used in test assertions. `sqlx::test` macro handles pool setup.

---

*Next: Plan 3 — Auth (registration, login, JWT, refresh tokens, replay detection, lockout, TOTP)*
