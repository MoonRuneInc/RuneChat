#!/usr/bin/env sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
cd "$SCRIPT_DIR"

COMPOSE=${COMPOSE:-"docker compose"}
ENV_FILE=${ENV_FILE:-".env.prod"}
COMPOSE_FILE=${COMPOSE_FILE:-"docker-compose.truenas.yml"}
BUILD_COMPOSE_FILE=${BUILD_COMPOSE_FILE:-"docker-compose.truenas-build.yml"}
APP_IMAGE=${RUNECHAT_APP_IMAGE:-"runechat-app:latest"}
FRONTEND_IMAGE=${RUNECHAT_FRONTEND_IMAGE:-"runechat-frontend:latest"}

usage() {
  cat <<'EOF'
RuneChat TrueNAS helper

Usage:
  ./truenas.sh init       Create .env.prod with generated secrets
  ./truenas.sh doctor     Validate Docker, env, compose, images, and local health
  ./truenas.sh images     Build and export image tarballs; requires full repo
  ./truenas.sh load       Load runechat-app.tar and runechat-frontend.tar if present
  ./truenas.sh config     Render Docker Compose config
  ./truenas.sh up         Start self-contained deploy from pre-built images
  ./truenas.sh up-build   Build images from source; requires full repo
  ./truenas.sh down       Stop containers
  ./truenas.sh logs       Follow logs
  ./truenas.sh status     Show container status
  ./truenas.sh clean      Stop containers and remove volumes

Run from the deploy/ directory on the TrueNAS host.
EOF
}

has_cmd() {
  command -v "$1" >/dev/null 2>&1
}

random_hex() {
  if has_cmd openssl; then
    openssl rand -hex "$1"
  else
    echo ""
  fi
}

random_base64() {
  if has_cmd openssl; then
    openssl rand -base64 "$1"
  else
    echo ""
  fi
}

ensure_docker() {
  if ! has_cmd docker; then
    echo "ERROR: docker is not installed or not on PATH." >&2
    exit 1
  fi

  if ! docker compose version >/dev/null 2>&1; then
    echo "ERROR: docker compose is unavailable." >&2
    exit 1
  fi
}

init_env() {
  if [ -f "$ENV_FILE" ]; then
    echo "$ENV_FILE already exists; leaving it unchanged."
    return
  fi

  jwt_secret=$(random_hex 64)
  totp_key=$(random_base64 32)

  cat > "$ENV_FILE" <<EOF
# RuneChat TrueNAS deployment environment.
# Fill DATABASE_URL before running ./truenas.sh up.

DATABASE_URL=
JWT_SECRET=$jwt_secret
TOTP_ENCRYPTION_KEY=$totp_key
DOMAIN=chat.moonrune.cc

REDIS_URL=redis://redis:6379
RUST_LOG=info

RUNECHAT_APP_IMAGE=$APP_IMAGE
RUNECHAT_FRONTEND_IMAGE=$FRONTEND_IMAGE
EOF

  chmod 600 "$ENV_FILE"
  echo "Created $ENV_FILE with generated JWT_SECRET and TOTP_ENCRYPTION_KEY."
  echo "Edit DATABASE_URL and DOMAIN before starting the stack."
}

read_env_value() {
  key="$1"
  if [ ! -f "$ENV_FILE" ]; then
    echo ""
    return
  fi

  grep -E "^${key}=" "$ENV_FILE" 2>/dev/null | tail -n 1 | cut -d= -f2- || true
}

require_env() {
  if [ ! -f "$ENV_FILE" ]; then
    echo "ERROR: $ENV_FILE not found. Run ./truenas.sh init first." >&2
    exit 1
  fi

  missing=0
  for key in DATABASE_URL JWT_SECRET TOTP_ENCRYPTION_KEY DOMAIN; do
    value=$(read_env_value "$key")
    if [ -z "$value" ]; then
      echo "ERROR: $key is empty in $ENV_FILE." >&2
      missing=1
    fi
  done

  if [ "$missing" -ne 0 ]; then
    exit 1
  fi
}

load_images() {
  ensure_docker
  loaded=0

  if [ -f runechat-app.tar ]; then
    docker load -i runechat-app.tar
    loaded=1
  fi

  if [ -f runechat-frontend.tar ]; then
    docker load -i runechat-frontend.tar
    loaded=1
  fi

  if [ "$loaded" -eq 0 ]; then
    echo "No image tarballs found. Expected runechat-app.tar and runechat-frontend.tar."
  fi
}

ensure_self_contained_images() {
  if ! docker image inspect "$APP_IMAGE" >/dev/null 2>&1; then
    echo "Missing backend image: $APP_IMAGE"
    echo "Run ./truenas.sh load if tarballs are present, or ./truenas.sh up-build from a full repo copy."
    exit 1
  fi

  if ! docker image inspect "$FRONTEND_IMAGE" >/dev/null 2>&1; then
    echo "Missing frontend image: $FRONTEND_IMAGE"
    echo "Run ./truenas.sh load if tarballs are present, or ./truenas.sh up-build from a full repo copy."
    exit 1
  fi
}

compose() {
  # shellcheck disable=SC2086
  $COMPOSE --env-file "$ENV_FILE" -f "$COMPOSE_FILE" "$@"
}

compose_build() {
  # shellcheck disable=SC2086
  $COMPOSE --env-file "$ENV_FILE" -f "$BUILD_COMPOSE_FILE" "$@"
}

wait_health() {
  if ! has_cmd curl; then
    echo "curl not found; skipping local health check."
    return
  fi

  i=1
  while [ "$i" -le 30 ]; do
    if curl -fsS http://localhost:8080/health >/dev/null 2>&1; then
      echo "Health check passed: http://localhost:8080/health"
      return
    fi
    sleep 2
    i=$((i + 1))
  done

  echo "Health check did not pass within 60 seconds. Run ./truenas.sh logs."
  exit 1
}

doctor() {
  ensure_docker
  require_env

  if [ ! -f prod.conf ]; then
    echo "ERROR: prod.conf is missing from deploy/." >&2
    exit 1
  fi

  if [ -f runechat-app.tar ] || [ -f runechat-frontend.tar ]; then
    echo "Image tarballs present."
  fi

  if docker image inspect "$APP_IMAGE" >/dev/null 2>&1; then
    echo "Backend image found: $APP_IMAGE"
  else
    echo "Backend image not loaded: $APP_IMAGE"
  fi

  if docker image inspect "$FRONTEND_IMAGE" >/dev/null 2>&1; then
    echo "Frontend image found: $FRONTEND_IMAGE"
  else
    echo "Frontend image not loaded: $FRONTEND_IMAGE"
  fi

  compose config >/dev/null
  echo "Compose config is valid."

  if has_cmd curl && curl -fsS http://localhost:8080/health >/dev/null 2>&1; then
    echo "Local health endpoint is reachable."
  else
    echo "Local health endpoint is not currently reachable. This is expected before ./truenas.sh up."
  fi
}

cmd=${1:-}
case "$cmd" in
  init)
    init_env
    ;;
  doctor)
    doctor
    ;;
  images)
    ./build-truenas-images.sh
    ;;
  load)
    load_images
    ;;
  config)
    ensure_docker
    require_env
    compose config
    ;;
  up)
    ensure_docker
    require_env
    [ -f runechat-app.tar ] || [ -f runechat-frontend.tar ] && load_images || true
    ensure_self_contained_images
    compose up -d
    wait_health
    ;;
  up-build)
    ensure_docker
    require_env
    if [ ! -f ../backend/Dockerfile ] || [ ! -f ../frontend/Dockerfile ]; then
      echo "ERROR: up-build requires a full repo copy with backend/ and frontend/ next to deploy/." >&2
      exit 1
    fi
    compose_build up --build -d
    wait_health
    ;;
  down)
    ensure_docker
    compose down
    ;;
  logs)
    ensure_docker
    compose logs -f
    ;;
  status)
    ensure_docker
    compose ps
    ;;
  clean)
    ensure_docker
    compose down -v
    ;;
  ""|-h|--help|help)
    usage
    ;;
  *)
    echo "Unknown command: $cmd" >&2
    usage >&2
    exit 1
    ;;
esac
