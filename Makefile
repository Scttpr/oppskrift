.PHONY: setup dev test lint fmt db migrate reset-db clean css css-watch

COMPOSE := $(shell command -v docker-compose 2>/dev/null || command -v podman-compose 2>/dev/null || echo "docker compose")
export DATABASE_URL ?= postgres://oppskrift:oppskrift@localhost:5432/oppskrift

# First-time setup
setup: db migrate
	@echo "Ready! Run 'make dev' to start"

# Development
dev: css
	cargo watch -x run

css:
	./tailwindcss -i static/css/input.css -o static/css/main.css --minify

css-watch:
	./tailwindcss -i static/css/input.css -o static/css/main.css --watch

# Database
db:
	@$(COMPOSE) up -d db 2>/dev/null || true
	@until $(COMPOSE) exec -T db pg_isready -U oppskrift >/dev/null 2>&1; do sleep 1; done

migrate: db
	@sqlx migrate run

reset-db: db
	sqlx database drop -y || true
	sqlx database create
	sqlx migrate run

# Quality
test: db migrate
	cargo test

lint: db migrate
	cargo clippy -- -D warnings
	cargo fmt -- --check

fmt:
	cargo fmt

# Cleanup
clean:
	cargo clean
	rm -f static/css/main.css
	$(COMPOSE) down -v
