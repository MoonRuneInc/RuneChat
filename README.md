# RuneChat

**A community chat platform that respects you.**

RuneChat is a free, open-source alternative to Discord — built under the [MoonRune](https://moonrune.cc) brand and designed around transparency, security, and community control from day one.

> Deploying at `chat.moonrune.cc`

---

## What you can do

- **Create a server** and invite your community with a shareable link
- **Organize conversations** into named channels — with spaces, capitalization, and real formatting, not `#forced-lowercase-slugs`
- **Chat in real time** with full message history and live delivery
- **Stay secure** — your session is protected against hijacking and account takeover by default
- **Take it with you** — native desktop app (Windows, macOS, Linux), Android, and a standard web build, all from the same codebase

---

## Governance

RuneChat gives communities meaningful control over their own spaces. Every server has a clear, transparent role structure — and the platform is designed to grow into full community self-governance over time.

### Server roles

Every server has three roles:

| Role | Who | What they can do |
|---|---|---|
| **Owner** | The person who created the server | Full control — manage members, promote admins, configure the server |
| **Admin** | Members promoted by the owner | Manage members and channels, moderate conversations |
| **Member** | Everyone else | Read and participate in channels they have access to |

Roles are explicit and visible. There are no hidden permission layers or opaque flags — if you have a role, you know what it means.

### Invite system

Invites are controlled by the server, not by the platform. Owners and admins can:
- Generate invite links with optional expiry times
- Set a maximum number of uses per link
- Revoke links at any time

There is no algorithm deciding who sees your server. Access is invite-only by default.

### Server succession (deadman protocol)

Servers shouldn't die because an owner's account gets locked.

Every server owner can configure a **succession plan** in server settings:

- **Designated successor** — a trusted admin who takes over management if the owner is locked out
- **Backup account** — a second account the owner personally controls, which the successor can transfer ownership to during recovery
- **Recovery hint** — a short question the owner sets in the app. The answer is **never entered into RuneChat** — the owner tells it to their designated successor verbally, in person or over a phone call. When the successor needs to transfer ownership, they ask the backup account holder the question out of band before proceeding.

> "Tell your designated successor the answer in person or over a voice call — never in a message. If someone claims to have recovered your account, your successor can ask them the question to confirm it's really you."

When a succession event is triggered (account compromised or suspended):
1. The owner is locked from server management actions — they can still read and send messages
2. The designated successor becomes acting caretaker and can manage the server
3. The successor can initiate an ownership transfer to the pre-registered backup account once they've verified identity out of band
4. When the original owner recovers their account, the succession event closes and they regain full control

No community should be held hostage to an account incident.

### Roadmap: community governance

The current role system is the foundation. Future milestones will add:

- **Moderation tooling** — transparent audit logs, member appeals, visible moderation history
- **Community voting** — structured proposals and votes for server decisions (rule changes, promotions, bans)
- **Platform governance** — RuneChat itself will adopt transparent governance for roadmap and policy decisions

These features are planned and scoped — not vague aspirations. The architecture is designed so they layer in cleanly without rearchitecting the core.

---

## Security

RuneChat treats your account security as a design requirement, not an afterthought.

- **Sessions expire quickly.** Access tokens live for 15 minutes, in memory only — never written to `localStorage` where a script could steal them.
- **Session theft is detectable.** Refresh tokens are single-use and rotate on every use. If a stolen token is replayed, RuneChat detects it immediately, kills all active sessions, and locks the account.
- **Compromised accounts are visible.** If a takeover is detected, a warning badge appears on the username everywhere on the platform — so your community knows your account may not be under your control while you're recovering it.
- **Recovery requires proof of identity.** Unlocking a compromised account requires TOTP (authenticator app) or email verification — not just a password reset.
- **2FA secrets are encrypted.** TOTP secrets are encrypted at rest with AES-256-GCM, not just hashed.

---

## Self-hosting

RuneChat runs as a single `docker compose up` and is designed to be self-hosted.

### Requirements

- Docker and Docker Compose
- A domain and TLS termination (for production)
- A managed PostgreSQL instance (for production — see below)

### Quick start

```bash
# 1. Clone the repo and copy the env template
git clone https://giteas.fullmooncyberworks.com/MoonRune/RuneChat.git
cd RuneChat
cp .env.example .env

# 2. Generate secrets (paste results into .env)
openssl rand -hex 64        # → JWT_SECRET
openssl rand -base64 32     # → TOTP_ENCRYPTION_KEY

# 3. Start everything
docker compose up --build
```

App available at **http://localhost:8080**.

### Environment variables

| Variable | Required | Notes |
|---|---|---|
| `JWT_SECRET` | Yes | `openssl rand -hex 64` |
| `TOTP_ENCRYPTION_KEY` | Yes | `openssl rand -base64 32` |
| `DATABASE_URL` | Yes | Pre-filled for local compose; use a managed DB for production |
| `REDIS_URL` | Yes | Pre-filled for local compose |
| `SMTP_HOST` / `SMTP_*` | No | Required for email OTP account unlock fallback |

See `.env.example` for full documentation.

### Production notes

- The bundled PostgreSQL image (`postgres:16-alpine`) is suitable for local development and internal use. For production, use a managed PostgreSQL service (Neon, Supabase, Railway, or RDS) and set `DATABASE_URL` accordingly.
- The `nginx/` directory contains a dev reverse proxy config. Replace with your own TLS-terminating config before exposing to the internet.

---

## Contributing

### Tech stack

| Layer | Technology |
|---|---|
| Backend | Rust · Axum · SQLx · Tokio |
| Frontend | TypeScript · React · Vite · Zustand · TanStack Query |
| Database | PostgreSQL 16 |
| Real-time | Redis pub/sub |
| Desktop / Mobile | Tauri v2 |
| Deployment | Docker Compose · Nginx |

### Development workflow

```bash
# Backend only (start DB and Redis first)
docker compose up -d db redis
cd backend && cargo run

# Backend tests
cd backend && cargo test

# Frontend only
cd frontend && npm install && npm run dev
```

### API

| Endpoint | Description |
|---|---|
| `POST /api/auth/register` | Create account |
| `POST /api/auth/login` | Log in |
| `POST /api/auth/refresh` | Rotate session |
| `POST /api/auth/logout` | Log out |
| `POST /api/auth/totp/enroll` | Set up authenticator app |
| `POST /api/auth/unlock/totp` | Unlock compromised account via TOTP |
| `POST /api/auth/unlock/email-otp/*` | Unlock via email OTP |
| `GET/POST /api/servers` | List / create servers |
| `GET/POST /api/servers/:id/invites` | Manage invites |
| `POST /api/invites/:code/join` | Join via invite |
| `GET/POST /api/channels` | Channels |
| `GET/POST /api/messages` | Message history / send |
| `GET /ws` | WebSocket — real-time messaging |

---

## License

FOSS — license to be determined before public release.
