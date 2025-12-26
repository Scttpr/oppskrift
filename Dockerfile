# Build stage
# Rust 1.85+ required for edition 2024 support (used by some dependencies)
FROM docker.io/library/rust:latest@sha256:97d17e8501a0b65f8b9b81bfbf3fac8ff76c0a348aefaf940d640fe15c3abfbf AS builder

WORKDIR /app

# Install dependencies for building
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/* \
    && apt-get clean

# Copy manifests
COPY Cargo.toml Cargo.lock* ./

# Create dummy src to cache dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -rf src

# Copy source code and SQLx offline data
COPY src ./src
COPY templates ./templates
COPY migrations ./migrations
COPY .sqlx ./.sqlx

# Touch main.rs to force rebuild
RUN touch src/main.rs

# Build the application (offline mode - no database needed)
ENV SQLX_OFFLINE=true
RUN cargo build --release

# Download Tailwind CSS v3 (project uses v3 syntax)
RUN curl -sLO https://github.com/tailwindlabs/tailwindcss/releases/download/v3.4.17/tailwindcss-linux-x64 \
    && chmod +x tailwindcss-linux-x64

COPY tailwind.config.js ./
COPY static ./static

RUN ./tailwindcss-linux-x64 -i static/css/input.css -o static/css/main.css --minify

# Runtime stage - same base as builder for glibc compatibility
FROM debian:trixie-slim

WORKDIR /app

# Install ca-certificates for HTTPS and create non-root user
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd -r -u 1000 -s /sbin/nologin appuser

# Copy binary and static files
COPY --from=builder /app/target/release/oppskrift /app/oppskrift
COPY --from=builder /app/static /app/static
COPY --from=builder /app/templates /app/templates
COPY --from=builder /app/migrations /app/migrations

# Set ownership and switch to non-root user
RUN chown -R appuser:appuser /app
USER appuser

EXPOSE 3000

ENV RUST_LOG=info
ENV HOST=0.0.0.0
ENV PORT=3000

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD /app/oppskrift --health-check || exit 1

# Security labels
LABEL org.opencontainers.image.source="https://github.com/scttpr/oppskrift"
LABEL org.opencontainers.image.description="Oppskrift - Federated recipe sharing"
LABEL org.opencontainers.image.licenses="AGPL-3.0-or-later"

ENTRYPOINT ["/app/oppskrift"]
