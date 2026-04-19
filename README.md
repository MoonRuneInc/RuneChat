# RuneChat

> A FOSS, security-first community chat platform — a Discord alternative built under the [MoonRune](https://moonrune.cc) brand.

**Target deployment:** `chat.moonrune.cc`

---

## Features

- **Servers & channels** — create servers, invite members, organize conversations into named channels
- **Real-time messaging** — WebSocket-backed live chat with full message history
- **Flexible channel names** — spaces, capitalization, readable formatting (slugs handled internally)
- **Security-first auth** — short-lived JWTs, rotating refresh tokens, replay-attack detection, TOTP 2FA
- **Compromise protection** — detected account takeovers are locked and visibly flagged to the community
- **Desktop + web + mobile** — Tauri v2 builds for Windows/macOS/Linux, Android, and a standard web build

---

## Stack

| Layer | Technology |
|---|---|
| Backend | Rust · Axum · SQLx · Tokio |
| Frontend | TypeScript · React · Vite · Zustand · TanStack Query |
| Database | PostgreSQL 16 |
| Real-time | Redis pub/sub |
| Desktop / Mobile | Tauri v2 |
| Deployment | Docker Compose · Nginx |

---

## Quick Start

```bash
# 1. Copy and fill in secrets
cp .env.example .env

# 2. Generate required values
openssl rand -hex 64          # → JWT_SECRET
openssl rand -base64 32       # → TOTP_ENCRYPTION_KEY

# 3. Start everything
docker compose up --build
```

App available at **http://localhost:8080**.

---

## Development

**Full stack (compose):**
```bash
docker compose up --build
```

**Backend only:**
```bash
docker compose up -d db redis
cd backend && cargo run
```

**Backend tests** (requires compose DB):
```bash
docker compose up -d db redis
cd backend && cargo test
```

**Frontend only:**
```bash
cd frontend
npm install
npm run dev
```

---

## Environment

| Variable | Required | How to generate |
|---|---|---|
| `JWT_SECRET` | Yes | `openssl rand -hex 64` |
| `TOTP_ENCRYPTION_KEY` | Yes | `openssl rand -base64 32` |
| `DATABASE_URL` | Yes | Pre-filled for local compose |
| `REDIS_URL` | Yes | Pre-filled for local compose |
| `SMTP_HOST` / `SMTP_*` | No | Required for email OTP account unlock |

See `.env.example` for all variables and comments.

---

## API Reference

| Endpoint | Description |
|---|---|
| `POST /api/auth/register` | Create account |
| `POST /api/auth/login` | Log in, receive JWT + refresh cookie |
| `POST /api/auth/refresh` | Rotate refresh token, get new JWT |
| `POST /api/auth/logout` | Revoke refresh token |
| `POST /api/auth/totp/enroll` | Begin TOTP enrollment |
| `POST /api/auth/totp/verify-enrollment` | Confirm TOTP setup |
| `POST /api/auth/unlock/totp` | Unlock compromised account via TOTP |
| `POST /api/auth/unlock/email-otp/*` | Unlock via email OTP (no TOTP fallback) |
| `GET/POST /api/servers` | List / create servers |
| `GET/POST /api/servers/:id/invites` | Manage invite links |
| `POST /api/invites/:code/join` | Join a server via invite |
| `GET/POST /api/channels` | List / create channels |
| `GET/POST /api/messages` | Message history / send |
| `GET /ws` | WebSocket — real-time messaging |
| `GET /health` | Health check |

---

## Security Model

RuneChat is designed with defense in depth. Key properties:

- **Short-lived JWTs** (15 min, in-memory only — never `localStorage`)
- **Rotating refresh tokens** stored as HMAC-SHA256 hashes, delivered as `httpOnly` + `Secure` + `SameSite=Strict` cookies
- **Replay detection** — a reused refresh token immediately kills all sessions and marks the account `compromised`
- **Compromised accounts** — login blocked; visible warning badge shown to other users; messages sent after the compromise timestamp are flagged
- **Account recovery** — TOTP (primary) or email OTP (fallback when no TOTP enrolled); requires 2FA to reactivate
- **TOTP secrets** encrypted at rest with AES-256-GCM
- **WebSocket auth** — JWT-verified at connect; compromised accounts rejected at connection time and excluded from message fan-out

---

## License

FOSS — license to be determined before public release.
