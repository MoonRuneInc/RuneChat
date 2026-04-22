# Cauldron TrueNAS SCALE Deploy

This directory is the TrueNAS deployment bundle. The easiest path is:

```bash
git clone https://github.com/MoonRuneInc/Cauldron.git
cd Cauldron/deploy
./truenas.sh init
```

Edit `.env.prod`, fill `DATABASE_URL`, confirm `DOMAIN`, then choose one start mode:

```bash
# If cauldron-app.tar and cauldron-frontend.tar are in deploy/
./truenas.sh up

# If you cloned the full repo and want TrueNAS to build the images
./truenas.sh up-build
```

The script validates config, starts the stack, and waits for:

```bash
http://localhost:8080/health
```

Expected response:

```json
{"status":"ok"}
```

If you want the portable image bundle first, run this from a full repo clone
on any Docker-capable build machine:

```bash
./deploy/build-truenas-images.sh
```

That creates:

```text
deploy/runechat-app.tar
deploy/runechat-frontend.tar
```

## What You Still Need

- TrueNAS SCALE with Docker Compose available.
- A managed PostgreSQL database. Neon works for staging.
- A TLS path to the host, either Cloudflare Tunnel or your existing reverse proxy.
- Port `8080` free on the TrueNAS host.

Do not deploy public production against the local development Postgres container. This bundle expects `DATABASE_URL` to point at managed or operator-owned Postgres.

## One-Command Tasks

Run from `deploy/`:

| Command | Purpose |
|---|---|
| `./truenas.sh init` | Create `.env.prod`, generate `JWT_SECRET`, generate `TOTP_ENCRYPTION_KEY` |
| `./truenas.sh doctor` | Validate Docker, env, compose config, images, and health |
| `./truenas.sh load` | Load `cauldron-app.tar` and `cauldron-frontend.tar` if present |
| `./truenas.sh up` | Start the self-contained pre-built image deployment |
| `./truenas.sh up-build` | Build from source and start; requires full repo clone |
| `./truenas.sh status` | Show container status |
| `./truenas.sh logs` | Follow stack logs |
| `./truenas.sh down` | Stop stack |
| `./truenas.sh clean` | Stop stack and remove volumes |

`make` wraps the same commands:

```bash
make init
make images
make doctor
make up
make logs
```

## Configure `.env.prod`

`./truenas.sh init` writes:

```env
DATABASE_URL=
JWT_SECRET=<generated>
TOTP_ENCRYPTION_KEY=<generated>
DOMAIN=chat.moonrune.cc
REDIS_URL=redis://redis:6379
RUST_LOG=info
CAULDRON_APP_IMAGE=cauldron-app:latest
CAULDRON_FRONTEND_IMAGE=cauldron-frontend:latest
```

Fill:

- `DATABASE_URL`: managed Postgres connection string, for example Neon:
  `postgresql://<user>:<pass>@<host>.neon.tech/cauldron?sslmode=require`
- `DOMAIN`: public hostname, for example `chat.moonrune.cc`

Leave `REDIS_URL` alone unless you are running Redis outside this compose stack.

## Deployment Modes

### Self-Contained Image Bundle

Use this when you want to build once, copy only the deploy bundle to TrueNAS,
and avoid building on the TrueNAS host.

On your build machine:

```bash
cd Cauldron/deploy
./truenas.sh init
# edit .env.prod
cp /path/to/cauldron-app.tar .
cp /path/to/cauldron-frontend.tar .
./truenas.sh up
```

The script loads tarballs automatically if they are present.

### Build From Source

Use this when the full repo is present on TrueNAS:

```bash
git clone https://github.com/MoonRuneInc/Cauldron.git
cd Cauldron/deploy
./truenas.sh init
# edit .env.prod
./truenas.sh up-build
```

This uses `docker-compose.truenas-build.yml` and Docker build contexts from the repo.

### TrueNAS Custom App

Use `docker-compose.truenas-custom-app.yml` only when you want to paste compose into the TrueNAS Custom App UI.

You must provide:

- `CAULDRON_APP_IMAGE`
- `CAULDRON_FRONTEND_IMAGE`
- `DATABASE_URL`
- `JWT_SECRET`
- `TOTP_ENCRYPTION_KEY`
- `DOMAIN`
- `NGINX_CONFIG_PATH`

Bind-mount `deploy/prod.conf` as the nginx config.

## Cloudflare Tunnel

Install `cloudflared` on the TrueNAS host:

```bash
curl -L https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-linux-amd64 \
  -o /usr/local/bin/cloudflared
chmod +x /usr/local/bin/cloudflared
```

Authenticate and create the tunnel:

```bash
cloudflared tunnel login
cloudflared tunnel create cauldron
```

Edit `cloudflared-config.yml`, replace `<TUNNEL-ID>`, then install it:

```bash
mkdir -p ~/.cloudflared
cp cloudflared-config.yml ~/.cloudflared/config.yml
cloudflared tunnel route dns cauldron chat.moonrune.cc
cloudflared service install
systemctl enable cloudflared
systemctl start cloudflared
```

Verify:

```bash
curl https://chat.moonrune.cc/health
```

## Existing Reverse Proxy

If you already run Nginx Proxy Manager, Zoraxy, Caddy, or another TLS proxy:

| Setting | Value |
|---|---|
| Upstream scheme | `http` |
| Upstream host | TrueNAS host IP |
| Upstream port | `8080` |
| WebSockets | enabled |
| TLS | terminate at the proxy |

Check both:

```bash
curl http://localhost:8080/health
curl https://chat.moonrune.cc/health
```

## Files

| File | Purpose |
|---|---|
| `truenas.sh` | Operator helper script |
| `build-truenas-images.sh` | Builds and exports the backend/frontend image tarballs |
| `Makefile` | Short wrappers around `truenas.sh` |
| `docker-compose.truenas.yml` | Self-contained deployment using pre-built images |
| `docker-compose.truenas-build.yml` | Full-repo source build deployment |
| `docker-compose.truenas-custom-app.yml` | TrueNAS Custom App compose template |
| `prod.conf` | nginx proxy config |
| `cloudflared-config.yml` | Cloudflare Tunnel template |

## Troubleshooting

### `DATABASE_URL is empty`

Run:

```bash
./truenas.sh init
nano .env.prod
```

Fill the managed Postgres connection string.

### Missing `cauldron-app:latest`

Use one of these:

```bash
./truenas.sh load
./truenas.sh up-build
```

`load` requires image tarballs. `up-build` requires the full repo.

### Local health fails

Run:

```bash
./truenas.sh status
./truenas.sh logs
```

Common causes:

- Postgres URL is wrong or unreachable from TrueNAS.
- Secret values were left blank.
- Port `8080` is already in use.
- Images were not loaded and source build was not used.

### Web works but real-time chat fails

Enable WebSocket support in the upstream proxy. `/ws` must be forwarded to port `8080`.
