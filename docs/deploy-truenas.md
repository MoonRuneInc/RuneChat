# Deploying RuneChat on TrueNAS SCALE

This guide covers deploying RuneChat as a Custom App on **TrueNAS SCALE 24.10 (Electric Eel) or later**, which natively supports Docker Compose. RuneChat runs as a self-contained stack (backend, PostgreSQL, Redis) proxied through your existing reverse proxy.

---

## Prerequisites

- TrueNAS SCALE 24.10+ (Electric Eel or later)
- A reverse proxy already running (e.g. Nginx Proxy Manager, Zoxary, Caddy) handling TLS/Let's Encrypt
- A domain pointed at your home/server IP
- Port 8080 free on your TrueNAS host

---

## Step 1 — Prepare storage datasets

Create persistent storage for the database and cache on your TrueNAS pool before deploying. In the TrueNAS UI:

**Storage → Create Dataset** (repeat for each):

| Dataset path | Purpose |
|---|---|
| `/mnt/<pool>/runechat/postgres` | PostgreSQL data |
| `/mnt/<pool>/runechat/redis` | Redis data |

Replace `<pool>` with your actual pool name (e.g. `tank`, `data`).

---

## Step 2 — Generate secrets

On any machine with OpenSSL:

```bash
openssl rand -hex 64        # → JWT_SECRET
openssl rand -base64 32     # → TOTP_ENCRYPTION_KEY
openssl rand -base64 24     # → POSTGRES_PASSWORD
```

Keep these somewhere safe — you'll paste them into the app environment in Step 4.

---

## Step 3 — Add the Custom App

In the TrueNAS UI:

1. Go to **Apps → Discover Apps → Custom App**
2. Set the app name to `runechat`
3. Paste the following into the **Compose** field:

```yaml
services:
  backend:
    image: ghcr.io/moonrune/runechat-backend:dev
    restart: unless-stopped
    ports:
      - "8080:8080"
    environment:
      DATABASE_URL: postgresql://runechat:${POSTGRES_PASSWORD}@db:5432/runechat
      REDIS_URL: redis://redis:6379
      JWT_SECRET: ${JWT_SECRET}
      TOTP_ENCRYPTION_KEY: ${TOTP_ENCRYPTION_KEY}
      RUST_LOG: info
    depends_on:
      db:
        condition: service_healthy
      redis:
        condition: service_started

  db:
    image: postgres:16-alpine
    restart: unless-stopped
    environment:
      POSTGRES_USER: runechat
      POSTGRES_PASSWORD: ${POSTGRES_PASSWORD}
      POSTGRES_DB: runechat
    volumes:
      - /mnt/<pool>/runechat/postgres:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U runechat"]
      interval: 5s
      timeout: 5s
      retries: 5

  redis:
    image: redis:7-alpine
    restart: unless-stopped
    volumes:
      - /mnt/<pool>/runechat/redis:/data
```

> Replace `/mnt/<pool>/runechat/...` with your actual dataset paths from Step 1.

---

## Step 4 — Set environment variables

In the Custom App UI, add the following environment variables using the secrets you generated in Step 2:

| Variable | Value |
|---|---|
| `POSTGRES_PASSWORD` | your generated password |
| `JWT_SECRET` | your generated JWT secret |
| `TOTP_ENCRYPTION_KEY` | your generated encryption key |

> These are injected at runtime and never stored in the Compose file — keep them out of any config files you commit or share.

---

## Step 5 — Deploy

Click **Save** / **Deploy**. TrueNAS will pull the images and start all three containers.

Verify in the Apps dashboard that all containers show as **Running** and the `db` healthcheck passes before proceeding.

---

## Step 6 — Configure your reverse proxy

Add a new proxy host pointing at your TrueNAS IP on port 8080.

**Example (Nginx Proxy Manager / Zoxary):**

| Setting | Value |
|---|---|
| Domain | `chat.yourdomain.com` |
| Scheme | `http` |
| Forward IP | `<TrueNAS IP>` |
| Forward Port | `8080` |
| **Websockets Support** | ✅ **Must be enabled** |

Enable **Force SSL** and request a Let's Encrypt certificate on the SSL tab.

> WebSocket support is required for real-time messaging. If it is not enabled, the app will load but chat will not work.

---

## Step 7 — Update DNS

Point your domain's A record at your server's public IP. Allow a few minutes for propagation.

---

## Step 8 — Verify

```bash
curl https://chat.yourdomain.com/api/health
# Expected: 200 OK
```

Then open the app in a browser and run through the golden path:
1. Register an account
2. Create a server
3. Create a channel
4. Send a message — confirms WebSocket is working

---

## Updating RuneChat

To pull a new image version:

1. In TrueNAS Apps, find the `runechat` app
2. Click **Update** or pull the new image tag manually
3. Restart the app — PostgreSQL data persists in your datasets

---

## Troubleshooting

**App won't start / db unhealthy**
Check that the dataset paths exist and TrueNAS has read/write access to them.

**Chat loads but messages don't send / WebSocket errors in browser console**
WebSocket support is not enabled in your reverse proxy. Enable it and restart the proxy host.

**`/api/health` returns 502**
The backend container hasn't started yet or is waiting on the DB healthcheck. Wait 30 seconds and retry.

**Forgot a secret / need to rotate**
Update the environment variable in the Custom App settings and restart the app. JWT and TOTP secrets changing will invalidate existing sessions — users will need to log in again.
