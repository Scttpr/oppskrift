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

# Download and verify Tailwind CSS
RUN curl -sLO https://github.com/tailwindlabs/tailwindcss/releases/latest/download/tailwindcss-linux-x64 \
    && chmod +x tailwindcss-linux-x64

COPY tailwind.config.js ./
COPY static ./static

RUN ./tailwindcss-linux-x64 -i static/css/input.css -o static/css/main.css --minify

# Runtime stage - using distroless for minimal attack surface
FROM gcr.io/distroless/cc-debian12:nonroot@sha256:8bd01e54ae6c812f85280cd4c6b5f6561fac96be56ea3bcf85da343a30eb9b23

WORKDIR /app

# Copy binary and static files with proper ownership
COPY --from=builder --chown=nonroot:nonroot /app/target/release/oppskrift /app/oppskrift
COPY --from=builder --chown=nonroot:nonroot /app/static /app/static
COPY --from=builder --chown=nonroot:nonroot /app/templates /app/templates
COPY --from=builder --chown=nonroot:nonroot /app/migrations /app/migrations

# Use nonroot user (uid 65532)
USER nonroot:nonroot

EXPOSE 3000

ENV RUST_LOG=info
ENV HOST=0.0.0.0
ENV PORT=3000

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD ["/app/oppskrift", "--health-check"] || exit 1

# Security labels
LABEL org.opencontainers.image.source="https://github.com/scttpr/oppskrift"
LABEL org.opencontainers.image.description="Oppskrift - Federated recipe sharing"
LABEL org.opencontainers.image.licenses="AGPL-3.0-or-later"

ENTRYPOINT ["/app/oppskrift"]
