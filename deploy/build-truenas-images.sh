#!/usr/bin/env sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
REPO_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)

APP_IMAGE=${RUNECHAT_APP_IMAGE:-"runechat-app:latest"}
FRONTEND_IMAGE=${RUNECHAT_FRONTEND_IMAGE:-"runechat-frontend:latest"}
OUT_DIR=${OUT_DIR:-"$SCRIPT_DIR"}

usage() {
  cat <<'EOF'
RuneChat TrueNAS image builder

Builds the backend and frontend Docker images, then exports them as tarballs
for transfer to a TrueNAS host.

Usage:
  ./deploy/build-truenas-images.sh

Optional environment variables:
  RUNECHAT_APP_IMAGE=runechat-app:latest
  RUNECHAT_FRONTEND_IMAGE=runechat-frontend:latest
  OUT_DIR=deploy

Outputs:
  runechat-app.tar
  runechat-frontend.tar
EOF
}

has_cmd() {
  command -v "$1" >/dev/null 2>&1
}

require_docker() {
  if ! has_cmd docker; then
    echo "ERROR: docker is not installed or not on PATH." >&2
    exit 1
  fi
}

require_repo() {
  if [ ! -f "$REPO_ROOT/backend/Dockerfile" ] || [ ! -f "$REPO_ROOT/frontend/Dockerfile" ]; then
    echo "ERROR: this script must live inside the full RuneChat repo." >&2
    echo "Expected backend/Dockerfile and frontend/Dockerfile next to deploy/." >&2
    exit 1
  fi
}

case "${1:-}" in
  "" )
    ;;
  -h|--help|help)
    usage
    exit 0
    ;;
  *)
    echo "Unknown argument: $1" >&2
    usage >&2
    exit 1
    ;;
esac

require_docker
require_repo
mkdir -p "$OUT_DIR"

echo "Building backend image: $APP_IMAGE"
docker build -t "$APP_IMAGE" -f "$REPO_ROOT/backend/Dockerfile" "$REPO_ROOT"

echo "Building frontend image: $FRONTEND_IMAGE"
docker build -t "$FRONTEND_IMAGE" -f "$REPO_ROOT/frontend/Dockerfile" "$REPO_ROOT/frontend"

echo "Exporting $APP_IMAGE to $OUT_DIR/runechat-app.tar"
docker save "$APP_IMAGE" -o "$OUT_DIR/runechat-app.tar"

echo "Exporting $FRONTEND_IMAGE to $OUT_DIR/runechat-frontend.tar"
docker save "$FRONTEND_IMAGE" -o "$OUT_DIR/runechat-frontend.tar"

echo "TrueNAS image bundle created:"
ls -lh "$OUT_DIR/runechat-app.tar" "$OUT_DIR/runechat-frontend.tar"
