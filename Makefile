.PHONY: all build run dev css css-watch test clean seed reset-db db db-stop up rebuild down lint fmt audit check migrate

# Detect compose command
COMPOSE := $(shell command -v docker-compose 2>/dev/null || command -v podman-compose 2>/dev/null || echo "docker compose")
export DATABASE_URL ?= postgres://oppskrift:oppskrift@localhost:5432/oppskrift

# Default: just build
all: build

build:
	cargo build --release

run: css
	cargo run

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

db-stop:
	@$(COMPOSE) stop db

migrate: db
	@sqlx migrate run

# Docker
up:
	$(COMPOSE) build app
	$(COMPOSE) up -d --force-recreate

rebuild:
	$(COMPOSE) build --no-cache app
	$(COMPOSE) up -d --force-recreate

down:
	@$(COMPOSE) down

# Quality
lint: db migrate
	cargo clippy --all-features -- -D warnings
	cargo fmt -- --check

check: db migrate
	cargo check --all-features

test: db migrate
	cargo test --all-features

fmt:
	cargo fmt

audit:
	cargo audit

clean:
	cargo clean
	rm -f static/css/main.css

seed:
	cargo run -- --seed

reset-db: db
	sqlx database drop -y || true
	sqlx database create
	sqlx migrate run
	cargo run -- --seed
