# RuneChat MVP — Architecture Foundation Design

**Date:** 2026-04-17
**Author:** Iris Vale (Head of Planning)
**Status:** Approved — ready for implementation planning

---

## 1. Purpose

This document defines the foundational architecture for the RuneChat MVP. It covers the full tech stack, data model, authentication and session security, real-time messaging, channel and invite design, deployment strategy, and red team testing plan.

All key decisions are also logged in `03_Decisions.md`. This spec is the canonical planning document for Phase 1.

---

## 2. Project Goals (MVP)

RuneChat is a FOSS, security-first chat platform and Discord alternative, deployed at `chat.moonrune.cc`. Users must be able to:

- Register accounts with usernames
- Create and join servers via invite links
- Create and browse channels within a server
- Chat in real time with a clean modern UI

Non-MVP (deferred): E2EE, federation, bots, voice/video, governance, moderation dashboard, advanced admin tooling.

Key product differentiators over Discord:
- Flexible channel naming (spaces, capitalisation, natural formatting)
- Compromised account visibility to the community
- Built for maintainability and supportability from day one

---

## 3. Tech Stack

| Layer | Technology |
|---|---|
| Backend | Rust · Axum · Tokio · SQLx |
| Frontend | TypeScript · React · Vite |
| Client state | Zustand |
| Server state / caching | TanStack Query |
| Database | PostgreSQL |
| Real-time broker | Redis (pub/sub) |
| Password hashing | Argon2id |
| Deployment | Docker Compose |
| Reverse proxy | Nginx |

**Frontend performance constraints:**
- No heavy UI component library — lightweight custom components with Tailwind (treeshaken at build)
- Virtual scrolling on all message lists (only DOM-renders visible messages)
- Lazy-loaded routes (minimal initial bundle)
- Zustand preferred over Redux for zero-boilerplate client state

---

## 4. Repository Structure

```
RuneChat/
├── backend/
│   ├── src/
│   │   ├── api/          # Axum route handlers (REST)
│   │   ├── auth/         # JWT, Argon2id, TOTP
│   │   ├── db/           # SQLx queries and domain models
│   │   ├── realtime/     # WebSocket handlers + Redis pub/sub
│   │   └── error.rs      # Unified error type
│   ├── migrations/       # SQLx migration files
│   └── Cargo.toml
├── frontend/
│   ├── src/
│   │   ├── components/
│   │   ├── pages/
│   │   ├── hooks/        # WebSocket, auth, query hooks
│   │   ├── stores/       # Zustand stores
│   │   └── api/          # Typed REST client
│   └── package.json
├── docs/
│   └── superpowers/
│       └── specs/
├── docker-compose.yml
├── .env.example
└── CLAUDE.md
```

---

## 5. Data Model

All tables in PostgreSQL. SQLx migrations manage schema.

```sql
-- Core identity
-- Requires: CREATE EXTENSION IF NOT EXISTS "citext"; in first migration
users (
  id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  username        CITEXT NOT NULL UNIQUE,   -- citext provides case-insensitive UNIQUE
  email           TEXT NOT NULL UNIQUE,
  password_hash   TEXT NOT NULL,           -- Argon2id
  account_status  TEXT NOT NULL DEFAULT 'active', -- active | compromised | suspended
  compromise_detected_at TIMESTAMPTZ,
  created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
)

-- 2FA
totp_secrets (
  id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  secret_encrypted TEXT NOT NULL,         -- AES-256-GCM encrypted; server must decrypt to verify codes
  enrolled_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
  verified_at     TIMESTAMPTZ            -- null until first successful use
)

-- Auth sessions
refresh_tokens (
  id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  token_hash      TEXT NOT NULL UNIQUE,   -- Argon2id hash of raw token
  expires_at      TIMESTAMPTZ NOT NULL,
  revoked_at      TIMESTAMPTZ,            -- null = active; set on use; replay = used token presented again
  created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
)
-- Replay detection: if token_hash exists AND revoked_at IS NOT NULL → replay attack
-- Cleanup: DELETE WHERE revoked_at IS NOT NULL AND expires_at < now() - interval '7 days'

-- Servers
servers (
  id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  name            TEXT NOT NULL,
  owner_id        UUID NOT NULL REFERENCES users(id),
  created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
)

server_members (
  server_id       UUID NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
  user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  role            TEXT NOT NULL DEFAULT 'member', -- owner | admin | member
  joined_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
  PRIMARY KEY (server_id, user_id)
)

-- Channels
channels (
  id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  server_id       UUID NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
  display_name    TEXT NOT NULL,           -- "General Discussion"
  slug            TEXT NOT NULL,           -- "general-discussion" (auto-derived)
  created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE (server_id, slug)
)

-- Messages
messages (
  id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  channel_id      UUID NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
  author_id       UUID NOT NULL REFERENCES users(id),
  content         TEXT NOT NULL,
  compromised_at_send BOOLEAN NOT NULL DEFAULT false, -- true if author was compromised when sent
  created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
  edited_at       TIMESTAMPTZ
)

-- Invites
invites (
  id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  server_id       UUID NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
  creator_id      UUID NOT NULL REFERENCES users(id),
  code            TEXT NOT NULL UNIQUE,    -- 8-char random alphanumeric
  max_uses        INTEGER,                 -- null = unlimited
  uses            INTEGER NOT NULL DEFAULT 0,
  expires_at      TIMESTAMPTZ,             -- null = never
  created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
)
```

---

## 6. Authentication & Session Security

### Flow

1. **Registration:** email + username + password → Argon2id hash stored
2. **Login:** credentials verified → access JWT issued (15-min expiry, in-memory client-side) + refresh token issued (7-day expiry, `httpOnly` cookie)
3. **Refresh:** client silently requests new access JWT before expiry; refresh token is rotated (old one invalidated, new one issued)
4. **Logout:** refresh token deleted server-side immediately

### Token hardening

| Property | Access JWT | Refresh Token |
|---|---|---|
| Storage | In-memory (Zustand) | `httpOnly` cookie |
| Expiry | 15 minutes | 7 days |
| Cookie flags | — | `Secure`, `SameSite=Strict`, `Path=/api/auth/refresh` |
| At-rest storage | Not stored | Argon2id hash only |
| Rotation | — | One-time use |

- `Cache-Control: no-store` on all auth endpoints
- Strict CSP: no inline scripts, no `eval`, scripts only from same origin
- Access JWT never written to `localStorage` or `sessionStorage`

### Replay attack response

If a refresh token is presented that has already been used:
1. All sessions for that user are immediately invalidated
2. `account_status` set to `compromised`
3. `compromise_detected_at` stamped
4. User cannot send new messages until unlocked

### Account lockout & compromise visibility

When an account is marked `compromised`:
- A visible badge appears next to the username in all server/channel contexts
- A banner is shown to other users: *"This account has been flagged as potentially compromised. The owner is regaining access."*
- Messages sent after `compromise_detected_at` are flagged with a visual indicator
- Account can be viewed but cannot send new messages

**Unlock mechanism:**
- Primary: valid TOTP code → `account_status` reset to `active`, markers cleared
- Fallback (no TOTP enrolled): email OTP → clears lockout, immediately prompts TOTP enrollment
- TOTP enrollment requires a successful first-use verification before being marked active

---

## 7. Real-time Architecture

### Connection lifecycle

1. Client authenticates (access JWT in-memory)
2. Client opens WebSocket to `/ws`, presents JWT in the initial handshake header
3. Backend validates JWT, registers connection in `Arc<DashMap<UserId, WsSender>>`
4. Backend subscribes to Redis channels for all servers the user is a member of

### Message flow

```
Client sends message
  → REST POST /channels/{id}/messages
  → Written to PostgreSQL (durable)
  → Published to Redis: channel:{channel_id}
  → All app instances subscribed to that Redis channel
  → Each instance fans out to local WebSocket connections with membership
```

Write-to-DB-first means Redis is a delivery mechanism only. If Redis drops, messages aren't lost — reconnecting clients replay via `GET /channels/{id}/messages?after={last_message_id}`.

### WebSocket event types (MVP)

- `message.created`
- `message.edited`
- `channel.created`
- `member.joined`
- `member.left`

---

## 8. Channel Naming

- **display_name:** User-typed. 1–80 characters. Allows spaces, capitalisation, punctuation. No control characters. Examples: `"General Discussion"`, `"Dev — Backend"`, `"Off Topic"`
- **slug:** Auto-derived on creation. Lowercase alphanumeric + hyphens. Max 80 chars. Auto-truncated if needed. Never user-typed, never shown directly.
- Slugs must be unique per server. Display names are not required to be unique.
- Slug derivation: lowercase → strip non-alphanumeric (except spaces/hyphens) → replace spaces with hyphens → collapse consecutive hyphens → trim leading/trailing hyphens

---

## 9. Username Rules

- 2–32 characters
- Allowed: letters, numbers, underscores, hyphens
- No spaces (display names may support spaces in a future profile expansion)
- Case-insensitive uniqueness: stored as entered, checked case-insensitively
- Validation errors are clear and specific (not generic "invalid username")

---

## 10. Invite System

- Codes: 8-char random alphanumeric (e.g. `X7kR2mQp`)
- Per-invite options: `max_uses` (nullable = unlimited), `expires_at` (nullable = never)
- `GET /invite/{code}` — previews server name and member count without requiring login
- Joining requires authentication
- Invites store: creator, creation time, use count — traceable for support
- Invalidated immediately when `max_uses` is reached or `expires_at` passes

---

## 11. Deployment

### Docker Compose services

```yaml
services:
  app:      # Rust/Axum binary
  frontend: # Nginx serving Vite build
  db:       # PostgreSQL
  redis:    # Redis
  proxy:    # Nginx reverse proxy → chat.moonrune.cc
```

- Redis and PostgreSQL are **not** exposed outside the Docker network
- TLS terminates at the proxy layer — the app never handles certificates directly
- This keeps the network overlay layer (Tailscale, WireGuard, etc.) trivially insertable later at the proxy without touching app code

### Environment variables (`.env.example`)

```
DATABASE_URL=
REDIS_URL=
JWT_SECRET=
JWT_EXPIRY_SECONDS=900
REFRESH_TOKEN_EXPIRY_DAYS=7
TOTP_ISSUER=RuneChat
TOTP_ENCRYPTION_KEY=        # AES-256-GCM key for TOTP secret storage
DOMAIN=chat.moonrune.cc
```

---

## 12. Red Team Testing Plan

To be executed before any public deployment. Covers all attack surfaces.

### Authentication & Tokens
- [ ] Brute force login — confirm rate limiting triggers and account locks correctly
- [ ] Credential stuffing simulation
- [ ] JWT manipulation: `alg:none` attack, key confusion, expiry tampering
- [ ] Refresh token replay: use a token twice — confirm all sessions killed and account flagged
- [ ] Attempt to read refresh token from JS — must be inaccessible via `httpOnly`
- [ ] Cache poisoning — confirm `Cache-Control: no-store` enforced on all auth endpoints
- [ ] CSRF against cookie endpoints — `SameSite=Strict` must block
- [ ] Cross-site WebSocket hijacking (CSWSH) — attempt WS from foreign origin

### Account Lockout & 2FA
- [ ] Brute force TOTP codes — confirm rate limiting and lockout after N attempts
- [ ] Race condition on unlock flow — two simultaneous valid TOTP submissions
- [ ] Attempt to bypass lockout using a valid pre-lockout access JWT
- [ ] Email OTP fallback abuse — confirm it cannot bypass TOTP requirement once enrolled

### Authorization & Access Control
- [ ] IDOR on servers, channels, messages — request resources from unjoined servers
- [ ] Privilege escalation: member attempting admin-only actions
- [ ] WebSocket channel subscription for servers/channels user is not a member of
- [ ] Invite code enumeration/brute force — confirm rate limiting
- [ ] Race condition on `max_uses` — parallel requests to exceed the limit

### Input & Injection
- [ ] XSS in display names, channel names, message content — CSP must block execution
- [ ] Unicode normalization attacks on usernames and slugs
- [ ] Oversized payloads on all input fields — confirm rejection, no crash
- [ ] SQL injection probes against all parameterized endpoints

### Infrastructure
- [ ] Confirm Redis is not accessible outside Docker network
- [ ] Confirm PostgreSQL is not externally accessible
- [ ] Environment variable leakage — no secrets in logs, error responses, or HTTP headers
- [ ] Confirm TLS enforced at proxy; plain HTTP access must redirect or be rejected

### Real-time
- [ ] Raw WebSocket frame injection — attempt to send messages without proper membership
- [ ] Attempt to subscribe to channels the user isn't a member of
- [ ] Connection flooding — confirm graceful degradation, no crash

---

## 13. Future Considerations (Not MVP)

- **VPN/network-layer security:** Tailscale or WireGuard as an access control tier, configurable at the proxy level. Enables per-instance or (future) per-server network restrictions for high-trust communities.
- **E2EE:** Architecture should not preclude adding end-to-end encryption to the message layer.
- **Federation:** Clean service boundaries make sharding and federation layerable later.
- **Audit log:** Token usage, login events, and privilege changes should eventually be surfaced to admins.

---

*This spec is the source of truth for Phase 1 implementation. All deviations require Iris Vale sign-off and a vault update.*
