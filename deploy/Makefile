export COMPOSE_ENV_FILES := .env.prod

.PHONY: up down logs config nginx-test clean

up:
	@test -f .env.prod || (echo "ERROR: .env.prod not found. Copy .env.prod.example to .env.prod and fill in required values." && exit 1)
	docker compose -f docker-compose.truenas.yml up -d

down:
	docker compose -f docker-compose.truenas.yml down

logs:
	docker compose -f docker-compose.truenas.yml logs -f

config:
	@test -f .env.prod || (echo "ERROR: .env.prod not found. Copy .env.prod.example to .env.prod and fill in required values." && exit 1)
	docker compose -f docker-compose.truenas.yml config

nginx-test:
	@test -f .env.prod || (echo "ERROR: .env.prod not found. Copy .env.prod.example to .env.prod and fill in required values." && exit 1)
	docker compose -f docker-compose.truenas.yml run --rm --no-deps proxy nginx -t

clean:
	docker compose -f docker-compose.truenas.yml down -v
