# RuneChat — TrueNAS SCALE Staging Deployment

> **Plan:** 09 — TrueNAS Staging Deploy
> **Target:** `chat.moonrune.cc` via Cloudflare Tunnel
> **Postgres:** Neon free tier
> **Scope:** Team testing and invite-only beta

---

## Prerequisites

- TrueNAS SCALE 24.04+ (Dragonfish) with Docker available
- Cloudflare account with `moonrune.cc` under management
- Neon account
- This `deploy/` directory copied to the TrueNAS host

---

## Quick Start (TrueNAS VM, self-contained)

This path uses pre-built images. You only need the files in `deploy/` on the host.

### 1. Copy deploy artifacts to TrueNAS

```bash
# From your local machine
scp deploy/* root@<truenas-ip>:/root/runechat-deploy/
ssh root@<truenas-ip>
cd /root/runechat-deploy
```

### 2. Load pre-built images

```bash
docker load -i runechat-app.tar
docker load -i runechat-frontend.tar
```

### 3. Provision Neon Database

1. Create a project at [console.neon.tech](https://console.neon.tech)
2. Create database `runechat`
3. Copy the connection string (it looks like):
   ```
   postgresql://<user>:<pass>@<host>.neon.tech/runechat?sslmode=require
   ```

### 4. Prepare `.env.prod`

```bash
cp .env.prod.example .env.prod
# Edit and fill in all required values
```

Required:
- `DATABASE_URL` — Neon connection string
- `JWT_SECRET` — `openssl rand -hex 64`
- `TOTP_ENCRYPTION_KEY` — `openssl rand -base64 32`
- `DOMAIN=chat.moonrune.cc`

### 5. Validate compose config

```bash
docker compose --env-file .env.prod -f docker-compose.truenas.yml config
```

Or, if you prefer Make:

```bash
make config
```

### 6. Start the stack

```bash
docker compose --env-file .env.prod -f docker-compose.truenas.yml up -d
```

Or with Make:

```bash
make up
```

### 7. Verify locally

```bash
curl http://localhost:8080/health
```

Expected: `{"status":"ok"}`

---

## Build from Source on TrueNAS

If you prefer to build the images on TrueNAS rather than loading pre-built `.tar` files:

1. Clone or copy the **entire repo** (not just `deploy/`) to TrueNAS.
2. Run from the repo root:
   ```bash
   docker compose -f deploy/docker-compose.truenas-build.yml up --build -d
   ```

This compose file uses parent-directory build contexts and bind-mounts `nginx/prod.conf` from the repo tree. It will not work if only `deploy/` is copied.

---

## Cloudflare Tunnel Setup

### Install cloudflared

```bash
curl -L https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-linux-amd64 \
  -o /usr/local/bin/cloudflared
chmod +x /usr/local/bin/cloudflared
```

### Authenticate

```bash
cloudflared tunnel login
```

This prints a URL. Open it in a browser, select `moonrune.cc`, authorize.

### Create tunnel

```bash
cloudflared tunnel create runechat
```

Copy the tunnel UUID from the output.

### Configure

Edit `cloudflared-config.yml` and replace `<TUNNEL-ID>` with the UUID:

```yaml
tunnel: <TUNNEL-ID>
credentials-file: /root/.cloudflared/<TUNNEL-ID>.json

ingress:
  - hostname: chat.moonrune.cc
    service: http://localhost:8080
  - service: http_status:404
```

Copy to the expected location:

```bash
cp cloudflared-config.yml ~/.cloudflared/config.yml
```

### Route DNS

```bash
cloudflared tunnel route dns runechat chat.moonrune.cc
```

### Run tunnel

```bash
# Foreground (for testing)
cloudflared tunnel run runechat

# Background service
cloudflared service install
systemctl enable cloudflared
systemctl start cloudflared
```

### Verify tunnel health

Check the [Cloudflare dashboard](https://dash.cloudflare.com) → Zero Trust → Tunnels.
The `runechat` tunnel should show as **Healthy**.

---

## Verify Public Access

```bash
# Health endpoint
curl https://chat.moonrune.cc/health

# WebSocket (manual browser test recommended)
# Open https://chat.moonrune.cc in a browser, register, create a server,
# send a message, confirm real-time delivery.
```

---

## TrueNAS Custom App (24.04+)

If your TrueNAS SCALE supports Custom Apps with Docker Compose:

1. Build images locally (or on TrueNAS):
   ```bash
   docker compose -f docker-compose.truenas-build.yml build
   ```

2. Tag and save:
   ```bash
   docker tag runechat-app:latest ghcr.io/<org>/runechat-app:staging
   docker tag runechat-frontend:latest ghcr.io/<org>/runechat-frontend:staging
   ```

3. Push to a registry (GHCR, Docker Hub, etc.) or load directly on TrueNAS.

4. In TrueNAS UI: **Apps** → **Custom App** → paste the contents of
   `docker-compose.truenas-custom-app.yml`.

5. Set environment variables in the TrueNAS UI.

6. Bind-mount `nginx/prod.conf` to `/etc/nginx/conf.d/default.conf`.

---

## Artifacts in this directory

| File | Purpose |
|---|---|
| `docker-compose.truenas.yml` | VM deployment — self-contained, uses pre-built images (needs `--env-file .env.prod`) |
| `docker-compose.truenas-build.yml` | VM deployment — builds from source (requires full repo) |
| `docker-compose.truenas-custom-app.yml` | Custom App deployment — uses pre-built images |
| `cloudflared-config.yml` | Cloudflare Tunnel ingress config |
| `prod.conf` | nginx production config (copy of `nginx/prod.conf`) |
| `runechat-app.tar` | Pre-built backend image (exported) |
| `runechat-frontend.tar` | Pre-built frontend image (exported) |

---

## Rollback

```bash
# Stop everything
docker compose --env-file .env.prod -f docker-compose.truenas.yml down -v

# Stop tunnel
systemctl stop cloudflared
```

---

## Troubleshooting

### `cloudflared` shows tunnel as Down
- Verify nginx is listening on `127.0.0.1:8080`
- Check `docker compose ps` — all containers should be `healthy`
- Check Cloudflare dashboard for tunnel errors

### `/health` returns 502
- Verify the `app` container is running: `docker compose logs app`
- Check `DATABASE_URL` is correct and Neon allows connections from the TrueNAS IP

### WebSocket fails to connect
- Verify `DOMAIN=chat.moonrune.cc` in `.env.prod`
- The backend WS handler checks `Origin: https://chat.moonrune.cc`
- Cloudflare Tunnel preserves the original Origin header

### Rate limits seem off
- With Cloudflare Tunnel, nginx sees `X-Forwarded-For` from Cloudflare
- `prod.conf` uses the `real_ip` module to set `$remote_addr` correctly
- Rate limiting keys should reflect actual client IPs
