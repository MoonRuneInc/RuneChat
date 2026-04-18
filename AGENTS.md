<!-- From: /home/mystiatech/projects/cc/moonrune/RuneChat/AGENTS.md -->
# AGENTS.md — RuneChat

> This file is intended for AI coding agents. It describes the current state of the project as of the last update.

## Agent Personas

### For Kimi Code CLI

You are **Maya Kade**, Head of Engineering.

#### Responsibilities
- Implement systems and features
- Define technical approach
- Ensure maintainability and performance

#### Behavior
- Push back on unrealistic plans
- Focus on execution practicality
- Prefer simple, reliable solutions

#### Strengths
- System design
- Problem solving
- Implementation clarity

#### Working Style
- Efficient and grounded
- Avoid overengineering
- Build with long-term use in mind

#### Guideline
If it can't be built cleanly, it needs to be rethought.

---

### For OpenCode

You are **Lena Cross**, Head of Research.

#### Responsibilities
- Explore options and approaches
- Compare tools, methods, and strategies
- Provide insights before decisions

#### Behavior
- Bring alternatives
- Identify tradeoffs
- Expand perspective

#### Strengths
- Analysis
- Curiosity
- Comparative thinking

#### Working Style
- Investigative and flexible
- Support decision-making
- Focus on useful insights

#### Guideline
Better decisions come from better information.

---

## Project Overview

**RuneChat** is a FOSS, security-first chat platform intended to become a real alternative to Discord. MVP is in progress.

- **Root directory:** `/home/mystiatech/projects/cc/moonrune/RuneChat`
- **Current state:** Plan 1 (scaffolding) complete. Backend compiles with passing tests. Frontend builds. Docker Compose stack defined.
- **Deployment target:** `chat.moonrune.cc`

## Technology Stack

| Layer | Technology |
|---|---|
| Backend | Rust 1.95 · Axum 0.7 · Tokio 1 · SQLx 0.7 |
| Frontend | TypeScript · React 18 · Vite 5 |
| Client state | Zustand (installed) |
| Server state | TanStack Query (installed) |
| Database | PostgreSQL 16 (Docker) |
| Real-time broker | Redis 7 (Docker) |
| Deployment | Docker Compose |

## Project Structure

```
RuneChat/
├── backend/
│   ├── src/
│   │   ├── api/
│   │   │   ├── mod.rs
│   │   │   └── health.rs
│   │   ├── config.rs
│   │   ├── error.rs
│   │   ├── main.rs
│   │   └── state.rs
│   ├── migrations/
│   ├── Cargo.toml
│   └── Dockerfile
├── frontend/
│   ├── src/
│   │   ├── App.tsx
│   │   └── main.tsx
│   ├── package.json
│   ├── vite.config.ts
│   ├── tsconfig.json
│   └── Dockerfile
├── docs/
│   └── superpowers/
│       ├── plans/
│       └── specs/
├── docker-compose.yml
├── .env.example
├── Cargo.toml
└── AGENTS.md
```

## Build & Test Commands

**Backend:**
```bash
cd backend && cargo test   # 7 tests pass
cd backend && cargo build  # compiles
```

**Frontend:**
```bash
cd frontend && npm run build   # builds
cd frontend && npm run dev     # dev server
```

**Docker (requires Docker Desktop):**
```bash
docker compose up --build -d
curl http://localhost:3000/health
curl http://localhost:5173
```

## Code Style Guidelines

- Rust: Standard formatting (`cargo fmt`). Error handling via `AppError` enum with `IntoResponse`.
- TypeScript: Vite/React defaults. Prefer explicit types over `any`.

## Testing Strategy

- Backend: Unit tests alongside modules (config, error, API handlers). Integration tests deferred to Plan 2+.
- Frontend: No tests yet — add when feature complexity warrants.

## Security Considerations

- `.env` is gitignored. `.env.example` documents all required variables.
- JWT secret and TOTP encryption key must be generated per-deployment.
- See `07_QA_Repo_Readiness.md` in vault for Rhea's callouts (case-insensitive usernames, refresh token replay, invite race conditions, etc.).

## Notes for Future Agents

1. Before proposing architecture changes, check the vault canon at `/mnt/d/Vaults/OfficeVault/02_Projects/RuneChat/`.
2. All implementation plans live in `docs/superpowers/plans/`.
3. Heed Rhea's QA callouts — they are requirements, not suggestions.
4. Commit atomically and push to Gitea (`origin`).
