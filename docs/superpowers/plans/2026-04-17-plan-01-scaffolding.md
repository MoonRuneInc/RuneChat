# Scaffolding Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Establish a compiling Rust/Axum backend skeleton, a Vite/React frontend skeleton, and a working Docker Compose stack with a passing health check — zero features, fully wired foundation.

**Architecture:** Rust workspace at the repo root containing a single `backend` crate. Frontend is a standalone Vite/React app in `frontend/`. Both are wired into `docker-compose.yml` with PostgreSQL and Redis. The backend exposes a single `GET /health` endpoint to confirm the stack is alive end-to-end.

**Tech Stack:** Rust 1.77+, Axum 0.7, Tokio 1, SQLx 0.7, Redis crate 0.24, Vite 5, React 18, TypeScript 5, Docker Compose v2

---

## File Map

**Create:**
- `Cargo.toml` — workspace root
- `backend/Cargo.toml` — backend crate manifest
- `backend/src/main.rs` — Axum entry point, app wiring
- `backend/src/config.rs` — env-var config struct
- `backend/src/error.rs` — unified `AppError` type
- `backend/src/api/mod.rs` — route registration
- `backend/src/api/health.rs` — health check handler
- `backend/src/state.rs` — `AppState` shared across handlers
- `backend/migrations/.gitkeep` — placeholder (migrations added in Plan 2)
- `frontend/package.json` — Vite/React deps
- `frontend/vite.config.ts` — dev proxy to backend on :3000
- `frontend/tsconfig.json`
- `frontend/index.html`
- `frontend/src/main.tsx`
- `frontend/src/App.tsx` — placeholder "RuneChat" page
- `docker-compose.yml` — app + db + redis + proxy services
- `.env.example` — all required env vars documented

---

## Task 1: Rust Workspace Root

**Files:**
- Create: `Cargo.toml`

- [ ] **Step 1: Create workspace Cargo.toml**

```toml
[workspace]
resolver = "2"
members = ["backend"]
```

- [ ] **Step 2: Verify workspace is valid**

```bash
cargo metadata --no-deps --manifest-path Cargo.toml
```

Expected: JSON output listing the `backend` member with no errors.

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml
git commit -m "feat: init rust workspace"
```

---

## Task 2: Backend Crate Manifest

**Files:**
- Create: `backend/Cargo.toml`

- [ ] **Step 1: Create backend/Cargo.toml**

```toml
[package]
name = "runechat-backend"
version = "0.1.0"
edition = "2021"

[dependencies]
axum = { version = "0.7", features = ["ws", "macros"] }
tokio = { version = "1", features = ["full"] }
sqlx = { version = "0.7", features = ["runtime-tokio-rustls", "postgres", "uuid", "time", "migrate"] }
redis = { version = "0.24", features = ["tokio-comp", "connection-manager"] }
tower = "0.4"
tower-http = { version = "0.5", features = ["cors", "trace", "set-header"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
uuid = { version = "1", features = ["v4", "serde"] }
time = { version = "0.3", features = ["serde"] }
dotenvy = "0.15"
thiserror = "1"
anyhow = "1"

[dev-dependencies]
axum-test = "14"
tokio = { version = "1", features = ["full"] }
```

- [ ] **Step 2: Create placeholder main.rs so it compiles**

```bash
mkdir -p backend/src
echo 'fn main() {}' > backend/src/main.rs
```

- [ ] **Step 3: Verify it compiles**

```bash
cargo build -p runechat-backend
```

Expected: Compiles with no errors (warnings about unused items are fine).

- [ ] **Step 4: Create migrations placeholder**

```bash
mkdir -p backend/migrations
touch backend/migrations/.gitkeep
```

- [ ] **Step 5: Commit**

```bash
git add backend/
git commit -m "feat: add backend crate manifest and deps"
```

---

## Task 3: Config Module

**Files:**
- Create: `backend/src/config.rs`

- [ ] **Step 1: Write the failing test**

Create `backend/src/config.rs`:

```rust
use std::num::ParseIntError;

#[derive(thiserror::Error, Debug)]
pub enum ConfigError {
    #[error("missing env var {0}: {1}")]
    Missing(String, std::env::VarError),
    #[error("invalid value for {0}: {1}")]
    Invalid(String, ParseIntError),
}

#[derive(Clone, Debug)]
pub struct Config {
    pub database_url: String,
    pub redis_url: String,
    pub jwt_secret: String,
    pub jwt_expiry_seconds: u64,
    pub refresh_token_expiry_days: u64,
    pub totp_issuer: String,
    pub totp_encryption_key: String,
    pub domain: String,
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        let get = |key: &str| -> Result<String, ConfigError> {
            std::env::var(key).map_err(|e| ConfigError::Missing(key.to_string(), e))
        };
        let get_u64 = |key: &str, default: u64| -> Result<u64, ConfigError> {
            match std::env::var(key) {
                Ok(v) => v.parse().map_err(|e| ConfigError::Invalid(key.to_string(), e)),
                Err(_) => Ok(default),
            }
        };

        Ok(Self {
            database_url: get("DATABASE_URL")?,
            redis_url: get("REDIS_URL")?,
            jwt_secret: get("JWT_SECRET")?,
            jwt_expiry_seconds: get_u64("JWT_EXPIRY_SECONDS", 900)?,
            refresh_token_expiry_days: get_u64("REFRESH_TOKEN_EXPIRY_DAYS", 7)?,
            totp_issuer: std::env::var("TOTP_ISSUER").unwrap_or_else(|_| "RuneChat".to_string()),
            totp_encryption_key: get("TOTP_ENCRYPTION_KEY")?,
            domain: std::env::var("DOMAIN").unwrap_or_else(|_| "chat.moonrune.cc".to_string()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_errors_on_missing_required_var() {
        std::env::remove_var("DATABASE_URL");
        let result = Config::from_env();
        assert!(matches!(result, Err(ConfigError::Missing(ref k, _)) if k == "DATABASE_URL"));
    }

    #[test]
    fn config_uses_defaults_for_optional_vars() {
        std::env::set_var("DATABASE_URL", "postgres://test");
        std::env::set_var("REDIS_URL", "redis://test");
        std::env::set_var("JWT_SECRET", "secret");
        std::env::set_var("TOTP_ENCRYPTION_KEY", "key");
        std::env::remove_var("JWT_EXPIRY_SECONDS");
        std::env::remove_var("REFRESH_TOKEN_EXPIRY_DAYS");
        std::env::remove_var("TOTP_ISSUER");
        std::env::remove_var("DOMAIN");

        let config = Config::from_env().unwrap();
        assert_eq!(config.jwt_expiry_seconds, 900);
        assert_eq!(config.refresh_token_expiry_days, 7);
        assert_eq!(config.totp_issuer, "RuneChat");
        assert_eq!(config.domain, "chat.moonrune.cc");
    }
}
```

- [ ] **Step 2: Add module to main.rs and run tests**

Replace `backend/src/main.rs` with:

```rust
mod config;

fn main() {}
```

```bash
cargo test -p runechat-backend config
```

Expected: 2 tests pass.

- [ ] **Step 3: Commit**

```bash
git add backend/src/config.rs backend/src/main.rs
git commit -m "feat: add config module with env-var loading and tests"
```

---

## Task 4: Error Type

**Files:**
- Create: `backend/src/error.rs`

- [ ] **Step 1: Write error.rs**

```rust
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};

#[derive(thiserror::Error, Debug)]
pub enum AppError {
    #[error("database error")]
    Database(#[from] sqlx::Error),
    #[error("not found")]
    NotFound,
    #[error("unauthorized")]
    Unauthorized,
    #[error("forbidden")]
    Forbidden,
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("internal error")]
    Internal(#[from] anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AppError::NotFound => (StatusCode::NOT_FOUND, self.to_string()),
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, self.to_string()),
            AppError::Forbidden => (StatusCode::FORBIDDEN, self.to_string()),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            AppError::Conflict(msg) => (StatusCode::CONFLICT, msg.clone()),
            AppError::Database(_) | AppError::Internal(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "internal server error".to_string())
            }
        };
        (status, Json(serde_json::json!({ "error": message }))).into_response()
    }
}

pub type Result<T> = std::result::Result<T, AppError>;

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;

    #[test]
    fn not_found_maps_to_404() {
        let resp = AppError::NotFound.into_response();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn unauthorized_maps_to_401() {
        let resp = AppError::Unauthorized.into_response();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn bad_request_maps_to_400() {
        let resp = AppError::BadRequest("missing field".to_string()).into_response();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn internal_maps_to_500_with_generic_message() {
        let resp = AppError::Internal(anyhow::anyhow!("secret internal detail")).into_response();
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }
}
```

- [ ] **Step 2: Add module to main.rs**

```rust
mod config;
mod error;

fn main() {}
```

- [ ] **Step 3: Run tests**

```bash
cargo test -p runechat-backend error
```

Expected: 4 tests pass.

- [ ] **Step 4: Commit**

```bash
git add backend/src/error.rs backend/src/main.rs
git commit -m "feat: add unified AppError type with IntoResponse and tests"
```

---

## Task 5: AppState

**Files:**
- Create: `backend/src/state.rs`

- [ ] **Step 1: Write state.rs**

```rust
use sqlx::PgPool;
use redis::aio::ConnectionManager;
use crate::config::Config;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub redis: ConnectionManager,
    pub config: Config,
}
```

- [ ] **Step 2: Add module to main.rs**

```rust
mod config;
mod error;
mod state;

fn main() {}
```

- [ ] **Step 3: Verify it compiles**

```bash
cargo build -p runechat-backend
```

Expected: Compiles with no errors.

- [ ] **Step 4: Commit**

```bash
git add backend/src/state.rs backend/src/main.rs
git commit -m "feat: add AppState with db, redis, and config"
```

---

## Task 6: Health Check Handler

**Files:**
- Create: `backend/src/api/mod.rs`
- Create: `backend/src/api/health.rs`

- [ ] **Step 1: Write failing test for health endpoint**

Create `backend/src/api/health.rs`:

```rust
use axum::{http::StatusCode, Json};
use serde_json::{json, Value};

pub async fn health_check() -> (StatusCode, Json<Value>) {
    (StatusCode::OK, Json(json!({ "status": "ok" })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request, Router};
    use axum_test::TestServer;

    fn health_router() -> Router {
        Router::new().route("/health", axum::routing::get(health_check))
    }

    #[tokio::test]
    async fn health_returns_200_with_ok_status() {
        let server = TestServer::new(health_router()).unwrap();
        let resp = server.get("/health").await;
        resp.assert_status_ok();
        let body: serde_json::Value = resp.json();
        assert_eq!(body["status"], "ok");
    }
}
```

Create `backend/src/api/mod.rs`:

```rust
pub mod health;

use axum::Router;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/health", axum::routing::get(health::health_check))
}
```

- [ ] **Step 2: Run failing test**

```bash
cargo test -p runechat-backend health
```

Expected: Test compiles and passes (handler has no dependencies to fail yet).

- [ ] **Step 3: Add api module to main.rs**

```rust
mod api;
mod config;
mod error;
mod state;

fn main() {}
```

- [ ] **Step 4: Verify compilation**

```bash
cargo build -p runechat-backend
```

Expected: Compiles with no errors.

- [ ] **Step 5: Commit**

```bash
git add backend/src/api/ backend/src/main.rs
git commit -m "feat: add health check endpoint with test"
```

---

## Task 7: Wire Axum App in main.rs

**Files:**
- Modify: `backend/src/main.rs`

- [ ] **Step 1: Replace main.rs with full wiring**

```rust
mod api;
mod config;
mod error;
mod state;

use config::Config;
use state::AppState;
use sqlx::postgres::PgPoolOptions;
use redis::aio::ConnectionManager;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = Config::from_env().map_err(|e| anyhow::anyhow!("{e}"))?;

    let db = PgPoolOptions::new()
        .max_connections(10)
        .connect(&config.database_url)
        .await?;

    sqlx::migrate!("./migrations").run(&db).await?;

    let redis_client = redis::Client::open(config.redis_url.clone())?;
    let redis = ConnectionManager::new(redis_client).await?;

    let state = AppState { db, redis, config };

    let app = api::router()
        .with_state(state)
        .layer(tower_http::trace::TraceLayer::new_for_http());

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    tracing::info!("listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await?;

    Ok(())
}
```

- [ ] **Step 2: Verify it compiles**

```bash
cargo build -p runechat-backend
```

Expected: Compiles with no errors. (It won't run without a real DB/Redis, which comes from Docker Compose.)

- [ ] **Step 3: Commit**

```bash
git add backend/src/main.rs
git commit -m "feat: wire axum app with db, redis, tracing in main"
```

---

## Task 8: Environment File

**Files:**
- Create: `.env.example`

- [ ] **Step 1: Create .env.example**

```bash
cat > .env.example << 'EOF'
# PostgreSQL
DATABASE_URL=postgres://runechat:runechat@db:5432/runechat

# Redis
REDIS_URL=redis://redis:6379

# JWT — generate with: openssl rand -hex 64
JWT_SECRET=

# Token expiry
JWT_EXPIRY_SECONDS=900
REFRESH_TOKEN_EXPIRY_DAYS=7

# TOTP — AES-256-GCM key, 32 bytes base64-encoded
# Generate with: openssl rand -base64 32
TOTP_ENCRYPTION_KEY=
TOTP_ISSUER=RuneChat

# Domain
DOMAIN=chat.moonrune.cc

# Logging
RUST_LOG=info
EOF
```

- [ ] **Step 2: Create local .env for development**

```bash
cp .env.example .env
# Fill in JWT_SECRET and TOTP_ENCRYPTION_KEY with generated values:
echo "JWT_SECRET=$(openssl rand -hex 64)" >> .env
echo "TOTP_ENCRYPTION_KEY=$(openssl rand -base64 32)" >> .env
```

- [ ] **Step 3: Verify .env is gitignored**

```bash
grep -q "^\.env$" .gitignore && echo "OK" || echo "MISSING — add .env to .gitignore"
```

Expected: `OK`

- [ ] **Step 4: Commit**

```bash
git add .env.example
git commit -m "feat: add .env.example with all required vars documented"
```

---

## Task 9: Docker Compose

**Files:**
- Create: `docker-compose.yml`

- [ ] **Step 1: Create docker-compose.yml**

```yaml
version: "3.9"

services:
  db:
    image: postgres:16-alpine
    environment:
      POSTGRES_USER: runechat
      POSTGRES_PASSWORD: runechat
      POSTGRES_DB: runechat
    volumes:
      - db_data:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U runechat"]
      interval: 5s
      timeout: 5s
      retries: 10

  redis:
    image: redis:7-alpine
    volumes:
      - redis_data:/data
    healthcheck:
      test: ["CMD", "redis-cli", "ping"]
      interval: 5s
      timeout: 5s
      retries: 10

  app:
    build:
      context: ./backend
      dockerfile: Dockerfile
    env_file: .env
    environment:
      DATABASE_URL: postgres://runechat:runechat@db:5432/runechat
      REDIS_URL: redis://redis:6379
    ports:
      - "3000:3000"
    depends_on:
      db:
        condition: service_healthy
      redis:
        condition: service_healthy

  frontend:
    build:
      context: ./frontend
      dockerfile: Dockerfile
    ports:
      - "5173:80"
    depends_on:
      - app

volumes:
  db_data:
  redis_data:
```

- [ ] **Step 2: Create backend Dockerfile**

Create `backend/Dockerfile`:

```dockerfile
FROM rust:1.77-alpine AS builder
RUN apk add --no-cache musl-dev pkgconfig openssl-dev
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
# Dummy build to cache dependencies
RUN mkdir src && echo "fn main(){}" > src/main.rs && cargo build --release && rm -rf src
COPY src ./src
COPY migrations ./migrations
RUN touch src/main.rs && cargo build --release

FROM alpine:3.19
RUN apk add --no-cache ca-certificates
COPY --from=builder /app/target/release/runechat-backend /usr/local/bin/runechat
EXPOSE 3000
CMD ["runechat"]
```

- [ ] **Step 3: Verify docker-compose config is valid**

```bash
docker compose config --quiet && echo "OK"
```

Expected: `OK` with no errors.

- [ ] **Step 4: Commit**

```bash
git add docker-compose.yml backend/Dockerfile
git commit -m "feat: add docker compose stack and backend dockerfile"
```

---

## Task 10: Frontend Scaffold

**Files:**
- Create: `frontend/package.json`, `frontend/vite.config.ts`, `frontend/tsconfig.json`, `frontend/index.html`, `frontend/src/main.tsx`, `frontend/src/App.tsx`

- [ ] **Step 1: Scaffold Vite React TypeScript project**

```bash
cd frontend
npm create vite@latest . -- --template react-ts
npm install
npm install zustand @tanstack/react-query
```

- [ ] **Step 2: Configure dev proxy to backend**

Replace `frontend/vite.config.ts`:

```typescript
import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

export default defineConfig({
  plugins: [react()],
  server: {
    proxy: {
      '/api': 'http://localhost:3000',
      '/ws': { target: 'ws://localhost:3000', ws: true },
    },
  },
})
```

- [ ] **Step 3: Replace App.tsx with RuneChat placeholder**

```typescript
export default function App() {
  return (
    <div style={{ fontFamily: 'sans-serif', padding: '2rem' }}>
      <h1>RuneChat</h1>
      <p>Coming soon.</p>
    </div>
  )
}
```

- [ ] **Step 4: Verify frontend builds**

```bash
cd frontend && npm run build
```

Expected: Vite build succeeds with no errors.

- [ ] **Step 5: Create frontend Dockerfile**

Create `frontend/Dockerfile`:

```dockerfile
FROM node:20-alpine AS builder
WORKDIR /app
COPY package*.json ./
RUN npm ci
COPY . .
RUN npm run build

FROM nginx:alpine
COPY --from=builder /app/dist /usr/share/nginx/html
EXPOSE 80
```

- [ ] **Step 6: Commit**

```bash
cd ..
git add frontend/
git commit -m "feat: scaffold vite/react frontend with backend proxy config"
```

---

## Task 11: End-to-End Stack Smoke Test

- [ ] **Step 1: Start the stack**

```bash
docker compose up --build -d
```

- [ ] **Step 2: Wait for health checks**

```bash
docker compose ps
```

Expected: All services show `healthy` or `running`. Wait up to 60s for `db` and `redis` to become healthy.

- [ ] **Step 3: Hit the health endpoint**

```bash
curl -s http://localhost:3000/health | python3 -m json.tool
```

Expected:
```json
{
    "status": "ok"
}
```

- [ ] **Step 4: Confirm frontend serves**

```bash
curl -s -o /dev/null -w "%{http_code}" http://localhost:5173
```

Expected: `200`

- [ ] **Step 5: Tear down**

```bash
docker compose down
```

- [ ] **Step 6: Commit**

```bash
git add .
git commit -m "chore: confirm end-to-end stack smoke test passes"
```

---

## Self-Review

**Spec coverage check:**

| Spec section | Covered |
|---|---|
| Tech stack (Rust/Axum/Tokio/SQLx, React/Vite, PostgreSQL, Redis, Docker Compose) | ✅ Tasks 1–11 |
| Repository structure (backend/, frontend/, docker-compose, .env.example) | ✅ Tasks 1–10 |
| Config from env (DATABASE_URL, REDIS_URL, JWT_SECRET, TOTP_ENCRYPTION_KEY, DOMAIN) | ✅ Task 3 |
| Unified error type | ✅ Task 4 |
| AppState (db + redis + config) | ✅ Task 5 |
| Data model | ❌ Deferred to Plan 2 (migrations) |
| Auth, real-time, channels, messages, frontend features | ❌ Deferred to Plans 3–7 |

**Placeholder scan:** No TBDs, TODOs, or vague steps. All code blocks are complete.

**Type consistency:** `AppState`, `Config`, `AppError`, `AppResult` used consistently. `health_check` signature matches registration in `api/mod.rs`.

---

*Next: Plan 2 — Database Foundations (SQLx migrations for all tables with Rhea's callouts)*
