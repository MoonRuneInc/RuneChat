<div align="center">

# 🌙 Cauldron

**A community chat platform that respects you.**

*Free. Open source. Security-first. Built under the [MoonRune](https://moonrune.cc) brand.*

[![Rust](https://img.shields.io/badge/backend-Rust-orange?style=flat-square&logo=rust)](https://www.rust-lang.org/)
[![React](https://img.shields.io/badge/frontend-React-61DAFB?style=flat-square&logo=react&logoColor=black)](https://react.dev/)
[![Tauri](https://img.shields.io/badge/desktop-Tauri_v2-FFC131?style=flat-square&logo=tauri&logoColor=black)](https://tauri.app/)
[![PostgreSQL](https://img.shields.io/badge/database-PostgreSQL_16-4169E1?style=flat-square&logo=postgresql&logoColor=white)](https://www.postgresql.org/)
[![Docker](https://img.shields.io/badge/deploy-Docker_Compose-2496ED?style=flat-square&logo=docker&logoColor=white)](https://docs.docker.com/compose/)
[![License](https://img.shields.io/badge/license-FOSS-green?style=flat-square)](#license)

> 🚀 Deploying at [`chat.moonrune.cc`](https://chat.moonrune.cc)

</div>

---

## 💬 What is Cauldron?

Cauldron is a free, open-source alternative to Discord built around three principles: **transparency**, **security**, and **community control**. No dark patterns. No opaque algorithms deciding who sees your server. No account incidents silently locking your community out.

---

## ✨ Features

### 💬 Messaging
- **Real-time chat** with full message history and live delivery
- **Flexible channel names** — spaces, capitalization, readable formatting. Not `#forced-lowercase-slugs`
- **Native apps** — Windows, macOS, Linux, and Android from a single Tauri codebase, plus a standard web build

### 🏘️ Community
- **Server-controlled invites** — generate links with optional expiry and use limits, revoke any time
- **Transparent roles** — Owner, Admin, Member. Explicit, visible, no hidden permission layers
- **Invite-only by default** — no algorithm deciding who finds your community

### 🔒 Security
- **Short-lived sessions** — access tokens expire in 15 minutes, stored in memory only, never `localStorage`
- **Replay attack detection** — refresh tokens are single-use and rotate. A replayed stolen token triggers immediate full session invalidation
- **Visible compromise warnings** — if a takeover is detected, a warning badge appears on the username platform-wide so your community knows
- **2FA-gated recovery** — unlocking a compromised account requires TOTP or email verification, not just a password reset
- **Encrypted 2FA secrets** — TOTP secrets are encrypted at rest with AES-256-GCM

### 🏛️ Governance
- **Server succession (deadman protocol)** — servers don't die because an owner's account gets locked. Configure a designated successor and backup account in server settings
- **Roadmap:** moderation tooling, community voting, and platform governance — scoped and architecturally planned, not vague aspirations

---

## 🔑 Server Succession

Every server owner can configure a **succession plan**:

| Setting | Purpose |
|---|---|
| 👤 **Designated successor** | A trusted admin who takes over management if the owner is locked out |
| 🔁 **Backup account** | A second account the owner controls — successor can transfer ownership here |
| 🗝️ **Recovery hint** | A question whose answer is communicated verbally, never stored in Cauldron |

> ⚠️ The answer to the recovery hint is **never entered into Cauldron**. Tell your designated successor in person or over a voice call. If someone claims to have recovered your account, the successor asks them the question out of band before proceeding.

When a succession event triggers:
1. 🔐 Owner is locked from server management (can still read and chat)
2. 🛡️ Designated successor becomes acting caretaker
3. 🔄 Successor initiates ownership transfer to the backup account after out-of-band identity verification
4. ✅ Owner recovers their account → succession event closes, full control restored

No community should be held hostage to an account incident.

---

## 🚀 Self-hosting

Cauldron is designed to self-host. One command gets you running.

### Requirements

- Docker and Docker Compose
- A domain with TLS termination (for production)

### Quick start

```bash
# 1. Clone and configure
git clone https://giteas.fullmooncyberworks.com/MoonRune/Cauldron.git
cd Cauldron
cp .env.example .env

# 2. Generate secrets
openssl rand -hex 64        # → JWT_SECRET
openssl rand -base64 32     # → TOTP_ENCRYPTION_KEY

# 3. Start
docker compose up --build
```

App available at **http://localhost:8080** 🎉

### Environment variables

| Variable | Required | Description |
|---|---|---|
| `JWT_SECRET` | ✅ | `openssl rand -hex 64` |
| `TOTP_ENCRYPTION_KEY` | ✅ | `openssl rand -base64 32` |
| `DATABASE_URL` | ✅ | Pre-filled for local compose |
| `REDIS_URL` | ✅ | Pre-filled for local compose |
| `SMTP_HOST` / `SMTP_*` | ➖ | Email OTP fallback for account unlock |

See `.env.example` for full documentation.

### Production notes

- The bundled `postgres:16-alpine` is suitable for local and internal use. For public production, use a managed PostgreSQL service or a dedicated container with persistent storage.
- Place a TLS-terminating reverse proxy (nginx, Caddy, or Zoxary) in front of port 8080 before exposing to the internet.
- ⚠️ Enable **WebSocket support** in your reverse proxy — required for real-time messaging.

### 🖥️ Deploying on TrueNAS SCALE

Running TrueNAS SCALE 24.10 or later? See the dedicated guide:
**[docs/deploy-truenas.md](docs/deploy-truenas.md)**

---

## 🛠️ Contributing

### Tech stack

| Layer | Technology |
|---|---|
| ⚙️ Backend | Rust · Axum · SQLx · Tokio |
| 🎨 Frontend | TypeScript · React · Vite · Zustand · TanStack Query |
| 🗄️ Database | PostgreSQL 16 |
| ⚡ Real-time | Redis pub/sub |
| 🖥️ Desktop / Mobile | Tauri v2 |
| 🐳 Deployment | Docker Compose · Nginx |

### Development workflow

```bash
# Start dependencies
docker compose up -d db redis

# Backend
cd backend && cargo run

# Backend tests (requires running DB)
cd backend && cargo test

# Frontend
cd frontend && npm install && npm run dev
```

### 📡 API reference

<details>
<summary>🔐 Auth</summary>

| Method | Endpoint | Description |
|---|---|---|
| `POST` | `/api/auth/register` | Create account |
| `POST` | `/api/auth/login` | Log in |
| `POST` | `/api/auth/refresh` | Rotate session |
| `POST` | `/api/auth/logout` | Log out |
| `POST` | `/api/auth/totp/enroll` | Set up authenticator app |
| `POST` | `/api/auth/unlock/totp` | Unlock compromised account via TOTP |
| `POST` | `/api/auth/unlock/email-otp/*` | Unlock via email OTP |

</details>

<details>
<summary>🏘️ Servers, Channels & Messages</summary>

| Method | Endpoint | Description |
|---|---|---|
| `GET` | `/api/servers` | List servers |
| `POST` | `/api/servers` | Create server |
| `GET` | `/api/servers/:id/invites` | List invites |
| `POST` | `/api/servers/:id/invites` | Create invite |
| `POST` | `/api/invites/:code/join` | Join via invite |
| `GET` | `/api/channels` | List channels |
| `POST` | `/api/channels` | Create channel |
| `GET` | `/api/messages` | Message history |
| `POST` | `/api/messages` | Send message |
| `GET` | `/ws` | WebSocket — real-time messaging |

</details>

---

## 📄 License

FOSS — license to be determined before public release.
