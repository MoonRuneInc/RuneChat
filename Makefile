# Cauldron — Production Deployment Commands
#
# These targets wrap docker compose with COMPOSE_ENV_FILES=.env.prod so the
# production stack can never accidentally pick up the development .env file.
#
# Required before first deploy:
#   cp .env.prod.example .env.prod
#   # fill in all required values
#
# Usage:
#   make prod-up        # build and start
#   make prod-down      # stop
#   make prod-logs      # tail logs
#   make prod-config    # validate compose config
#   make prod-clean     # stop and remove volumes

.PHONY: prod-up prod-down prod-logs prod-config prod-clean

# Force Compose to read .env.prod and ignore the default .env.
export COMPOSE_ENV_FILES := .env.prod

prod-up:
	@test -f .env.prod || (echo "ERROR: .env.prod not found. Copy from .env.prod.example and fill in required values." && exit 1)
	docker compose -f docker-compose.prod.yml up --build -d

prod-down:
	docker compose -f docker-compose.prod.yml down

prod-logs:
	docker compose -f docker-compose.prod.yml logs -f

prod-config:
	@test -f .env.prod || (echo "ERROR: .env.prod not found. Copy from .env.prod.example and fill in required values." && exit 1)
	docker compose -f docker-compose.prod.yml config

prod-nginx-test:
	@test -f .env.prod || (echo "ERROR: .env.prod not found. Copy from .env.prod.example and fill in required values." && exit 1)
	docker compose -f docker-compose.prod.yml run --rm --no-deps proxy nginx -t

prod-clean:
	docker compose -f docker-compose.prod.yml down -v
