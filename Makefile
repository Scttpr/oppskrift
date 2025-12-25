.PHONY: all build run dev css css-watch test clean seed reset-db

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

# Run tests
test:
	cargo test

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

# Lint and format
lint:
	cargo clippy -- -D warnings
	cargo fmt -- --check

# Format code
fmt:
	cargo fmt

# Security audit
audit:
	cargo audit
