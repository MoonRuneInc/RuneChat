# Plan 12 — Rebrand: RuneChat → Cauldron

**Author:** Iris Vale  
**Date:** 2026-04-22  
**Assigned to:** Maya Kade  
**QA:** Rhea Solis  

---

## Goal

Rename the product from RuneChat to Cauldron across all code, config, docs, and CI. The domain (`chat.moonrune.cc`) is unchanged. All technical identifiers (crate names, DB name, Docker image tags, Tauri app identifier) must match the new brand. Historical plan/spec docs in `docs/superpowers/` are audit trail — do not rename those files.

---

## String Mapping

| Old | New |
|---|---|
| `RuneChat` | `Cauldron` |
| `runechat` | `cauldron` |
| `runechat-backend` | `cauldron-backend` |
| `runechat_lib` | `cauldron_lib` |
| `cc.moonrune.runechat` | `cc.moonrune.cauldron` |
| `postgres://runechat:runechat@.../runechat` | `postgres://cauldron:cauldron@.../cauldron` |
| `POSTGRES_USER: runechat` | `POSTGRES_USER: cauldron` |
| `POSTGRES_PASSWORD: runechat` | `POSTGRES_PASSWORD: cauldron` |
| `POSTGRES_DB: runechat` | `POSTGRES_DB: cauldron` |
| `pg_isready -U runechat` | `pg_isready -U cauldron` |
| `TOTP_ISSUER=RuneChat` | `TOTP_ISSUER=Cauldron` |
| `SMTP_FROM=RuneChat <...>` | `SMTP_FROM=Cauldron <...>` |

---

## Tasks

### 1. Backend Rust crate

File: `backend/Cargo.toml`

- `name = "runechat-backend"` → `name = "cauldron-backend"`

File: `backend/src/main.rs` — search for any `RuneChat` strings (logging, comments).

File: `backend/src/config.rs` — search for any `runechat` defaults.

File: `backend/src/auth/email.rs` — likely contains brand name in email templates.

File: `backend/src/auth/tokens.rs` — check for brand strings.

File: `backend/src/pwned.rs` — check for brand strings.

Run `cargo check -p cauldron-backend` in backend after rename to verify compile.

---

### 2. Frontend Tauri crate

File: `frontend/src-tauri/Cargo.toml`

- `name = "runechat"` → `name = "cauldron"`
- `description = "RuneChat — ..."` → `description = "Cauldron — Cross-platform chat client"`
- `[lib] name = "runechat_lib"` → `name = "cauldron_lib"`

File: `frontend/src-tauri/src/lib.rs`

- `log::info!("RuneChat starting...")` → `log::info!("Cauldron starting...")`

File: `frontend/src-tauri/src/main.rs` — check for brand references.

After rename, `runechat_lib` must also be updated in any `#[cfg_attr]` or `use` statements in lib.rs/main.rs.

---

### 3. Tauri config

File: `frontend/src-tauri/tauri.conf.json`

- `"productName": "RuneChat"` → `"productName": "Cauldron"`
- `"identifier": "cc.moonrune.runechat"` → `"identifier": "cc.moonrune.cauldron"`
- `"title": "RuneChat"` (window title) → `"title": "Cauldron"`

---

### 4. Frontend React UI

File: `frontend/index.html` — page title.

File: `frontend/src/pages/RegisterPage.tsx` — any brand strings.

File: `frontend/src/pages/LoginPage.tsx` — any brand strings.

Search all `frontend/src/**/*.tsx` for `RuneChat` and replace.

---

### 5. Docker Compose files — dev

File: `docker-compose.yml`

- `POSTGRES_USER: runechat` → `cauldron`
- `POSTGRES_PASSWORD: runechat` → `cauldron`
- `POSTGRES_DB: runechat` → `cauldron`
- `pg_isready -U runechat` → `pg_isready -U cauldron`
- `DATABASE_URL: postgres://runechat:runechat@db:5432/runechat` → `postgres://cauldron:cauldron@db:5432/cauldron`

---

### 6. Docker Compose files — prod

File: `docker-compose.prod.yml` — apply same DB credential substitutions as above.

---

### 7. Env files

File: `.env.example`

- `DATABASE_URL=postgres://runechat:runechat@db:5432/runechat` → `cauldron`
- `TOTP_ISSUER=RuneChat` → `Cauldron`
- `SMTP_FROM=RuneChat <noreply@moonrune.cc>` → `Cauldron <noreply@moonrune.cc>`

File: `.env.prod.example` — same substitutions.

---

### 8. Deploy directory

Files to update (all `runechat`/`RuneChat` occurrences):

- `deploy/docker-compose.truenas.yml`
- `deploy/docker-compose.truenas-build.yml`
- `deploy/docker-compose.truenas-custom-app.yml`
- `deploy/build-truenas-images.sh`
- `deploy/truenas.sh`
- `deploy/cloudflared-config.yml`
- `deploy/README.md`

---

### 9. GitHub Actions workflows

Files: `.github/workflows/backend.yml`, `build.yml`, `release.yml`

Key replacements:
- `runechat-db:latest` → `cauldron-db:latest`
- `runechat-backend:latest` → `cauldron-backend:latest`
- `runechat-web` → `cauldron-web`
- `runechat-windows` → `cauldron-windows`
- `runechat-android` → `cauldron-android`
- `cargo check -p runechat` → `cargo check -p cauldron`
- DB env vars: `POSTGRES_USER`, `POSTGRES_DB`, `POSTGRES_PASSWORD`, `pg_isready -U`
- `DATABASE_URL` connection strings
- Release title: `RuneChat ${{ ... }}` → `Cauldron ${{ ... }}`

---

### 10. Gitea Actions workflows

Files: `.gitea/workflows/backend.yml`, `build.yml`, `release.yml`

Apply identical substitutions as GitHub Actions above.

---

### 11. GitHub Issue Templates

Files: `.github/ISSUE_TEMPLATE/bug_report.yml`, `feature_request.yml`, `config.yml`

- Replace all `RuneChat` brand references.
- Update GitHub URLs from `MoonRuneInc/RuneChat` to `MoonRuneInc/Cauldron` (repo will be renamed by user).

---

### 12. Root and top-level docs

Files: `README.md`, `ORG_README.md`, `CHANGELOG.md`, `AGENTS.md`, `CODEX.md`, `CLAUDE.md`

Replace all `RuneChat`/`runechat` occurrences.

Note: `CLAUDE.md` contains vault references — update brand name in text but do not change vault paths (those will be updated separately).

---

### 13. Docs directory

Files to update:
- `docs/deploy-truenas.md`
- `docs/release-checklist.md`
- `docs/public-deployment-security-checklist.md`

File to rename:
- `docs/forum-post-runechat.md` → `docs/forum-post-cauldron.md`
- Update internal content too.

Do NOT rename files in `docs/superpowers/` — those are historical audit records.

---

### 14. Makefile

File: `Makefile` — update header comment `RuneChat — Production Deployment Commands` → `Cauldron — Production Deployment Commands`.

---

### 15. Redteam suite

Files: `redteam/conftest.py`, `redteam/README.md`, `redteam/rtlib/client.py`
Files: `redteam/tests/*.py`

Replace any `RuneChat`/`runechat` brand strings. DB connection strings if present.

---

### 16. Cargo.lock

After all `Cargo.toml` renames, run `cargo generate-lockfile` (or `cargo check`) from the workspace root to regenerate `Cargo.lock` with updated crate names.

---

### 17. Vault update (Iris Vale, not Maya)

After Maya's work is committed, Iris will update:
- `00_Overview.md` — product name
- `01_Status.md` — all RuneChat references
- `02_Tasks.md` — mark task complete
- `03_Decisions.md` — any RuneChat references

---

## Verification Checklist (Rhea)

- [ ] `grep -r "RuneChat\|runechat" . --include="*.rs" --include="*.toml" --include="*.json" --include="*.yml" --include="*.yaml" --include="*.md" --include="*.tsx" --include="*.html" --include="*.sh" --include="*.py" --include="*.env*" --include="Makefile"` returns no hits outside `docs/superpowers/` historical docs and `.git/`
- [ ] `cargo check` passes in backend (crate `cauldron-backend`)
- [ ] `cargo check` passes in `frontend/src-tauri` (crate `cauldron`)
- [ ] `docker compose build` succeeds with renamed DB credentials
- [ ] `docker compose up -d` starts cleanly; `docker compose ps` shows all healthy
- [ ] `curl localhost:8080/health` returns 200
- [ ] Backend tests pass: `DATABASE_URL=postgres://cauldron:cauldron@localhost:5432/cauldron cargo test -p cauldron-backend`

---

## Out of Scope

- Domain changes (stays `chat.moonrune.cc`)
- Gitea/GitHub repo rename (handled by user)
- DB migration files (they define schema structure, not brand names — no changes needed)
- Historical docs in `docs/superpowers/plans/`, `docs/superpowers/specs/`, `docs/superpowers/reviews/`
