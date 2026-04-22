# RuneChat — A FOSS Discord Alternative We're Building

Hey all,

So I've been building a Discord alternative called **RuneChat** and figured it's time to actually talk about it.

---

## Why though

Discord works, mostly. But a few things have bugged me for a while:

- Channel names are stuck in `#lowercase-hyphen-jail` for no good reason
- If your account gets compromised, your community has no idea — Discord just silently lets whoever stole it keep chatting as you
- If a server owner's account gets locked, the whole community is basically held hostage until they sort it out
- No self-hosting. Your community lives on their servers, on their terms.

None of it is catastrophic on its own, but it adds up. So I started building something better.

---

## What RuneChat is

It's a self-hostable community chat platform — servers, channels, invites, real-time messaging, clean modern UI. The core Discord use case, but built from scratch with different priorities.

### The security stuff (this is the part I'm most proud of)

- Sessions expire fast (15 min) and live in memory only — no `localStorage` where scripts can grab them
- Refresh tokens are **single-use**. If a stolen token gets replayed, RuneChat catches it instantly, kills every active session, and locks the account
- **Compromised accounts get a visible warning badge** — your community sees it everywhere on the platform before you've even noticed something's wrong
- Recovering a locked account requires TOTP or email verification, not just a password reset
- Passwords get checked against the HaveIBeenPwned breach database at registration — using k-anonymity, so the full password never leaves your server. If it's been in a breach, it gets rejected before it's ever stored
- TOTP secrets are encrypted at rest with AES-256-GCM

### Channel names

Channels support spaces, capitalization, whatever. `# General Chat` instead of `#general-chat`. The internal slug is handled behind the scenes. It's a small thing but it's been on my wishlist for years.

### Server succession

This one's a bit unique. Every server owner can set up a **succession plan** — a designated successor (trusted admin), a backup account, and a recovery hint. The hint's answer gets communicated verbally, never typed into RuneChat. If the owner's account gets locked, the successor steps in as caretaker and can transfer ownership to the backup account after verifying identity out of band.

No community should die because one account had a bad day.

### Self-hosting

```bash
git clone ...
cp .env.example .env
docker compose up --build
```

Runs on anything with Docker. There's also a full guide for deploying on TrueNAS SCALE if that's your setup.

---

## Tech stack

- **Backend:** Rust (Axum, SQLx, Tokio) — memory safety matters for a platform people trust with their conversations
- **Frontend:** TypeScript + React + Vite + Zustand
- **Desktop/Mobile:** Tauri v2 — one codebase for Windows, macOS, Linux, and Android
- **Database:** PostgreSQL 16
- **Real-time:** Redis pub/sub
- **Deployment:** Docker Compose

---

## Where we're at

MVP is functionally complete — accounts, servers, invites, channels, real-time chat all work. Security layer is in and QA-cleared (we ran a full Red Team suite against it). Currently in team testing, working toward an invite-only beta.

Still on the list before open public launch:
- Server succession UI
- Frontend automated test harness
- A few CI release workflow bits

---

## It's fully open source

Everything is on our Gitea. License is being finalised before public release but it's going full FOSS — fork it, self-host it, contribute to it.

If you spot something in the security model that looks off, please say so. That's exactly the kind of feedback I want.

---

Repo: `https://giteas.fullmooncyberworks.com/MoonRune/RuneChat`
Live at: `chat.moonrune.cc` (soon)

Happy to answer questions!
