.PHONY: all build run dev css css-watch test clean seed reset-db db db-stop lint fmt audit check

# Default target
all: css build

# Build the Rust application
build:
	cargo build --release

# Run the application
run: css
	cargo run

# Development mode with hot reload
dev:
	cargo watch -x run

# Build Tailwind CSS
css:
	./tailwindcss -i static/css/input.css -o static/css/main.css --minify

# Watch Tailwind CSS for changes
css-watch:
	./tailwindcss -i static/css/input.css -o static/css/main.css --watch

# Run tests (ensures database is running)
test: db
	@sqlx migrate run --source migrations > /dev/null 2>&1 || true
	cargo test --all-features

# Clean build artifacts
clean:
	cargo clean
	rm -f static/css/main.css

# Seed the database (development)
seed:
	cargo run -- --seed

# Reset database (drop, migrate, seed)
reset-db:
	sqlx database drop -y || true
	sqlx database create
	sqlx migrate run
	cargo run -- --seed

# Database URL for local development
export DATABASE_URL ?= postgres://oppskrift:oppskrift@localhost:5432/oppskrift

# Detect compose command (docker-compose, podman-compose, or docker compose)
COMPOSE := $(shell command -v docker-compose 2>/dev/null || command -v podman-compose 2>/dev/null || echo "docker compose")

# Start database container
db:
	@$(COMPOSE) up -d db
	@echo "Waiting for database to be ready..."
	@until $(COMPOSE) exec -T db pg_isready -U oppskrift > /dev/null 2>&1; do sleep 1; done
	@echo "Database is ready"

# Stop database container
db-stop:
	$(COMPOSE) stop db

# Run migrations (starts db if needed)
migrate: db
	sqlx migrate run

# Lint and format (ensures database is running for SQLx)
lint: db
	@sqlx migrate run --source migrations > /dev/null 2>&1 || true
	cargo clippy --all-features -- -D warnings
	cargo fmt -- --check

# Quick check (compile only, with database)
check: db
	@sqlx migrate run --source migrations > /dev/null 2>&1 || true
	cargo check --all-features

# Format code
fmt:
	cargo fmt

# Security audit
audit:
	cargo audit
