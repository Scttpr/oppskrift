# Build stage
FROM rust:1.75-bookworm AS builder

WORKDIR /app

# Install dependencies for building
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests
COPY Cargo.toml Cargo.lock* ./

# Create dummy src to cache dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -rf src

# Copy source code
COPY src ./src
COPY templates ./templates
COPY migrations ./migrations

# Touch main.rs to force rebuild
RUN touch src/main.rs

# Build the application
RUN cargo build --release

# Download and run Tailwind CSS
RUN curl -sLO https://github.com/tailwindlabs/tailwindcss/releases/latest/download/tailwindcss-linux-x64 \
    && chmod +x tailwindcss-linux-x64

COPY tailwind.config.js ./
COPY static ./static

RUN ./tailwindcss-linux-x64 -i static/css/input.css -o static/css/main.css --minify

# Runtime stage
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Copy binary and static files
COPY --from=builder /app/target/release/oppskrift /app/oppskrift
COPY --from=builder /app/static /app/static
COPY --from=builder /app/templates /app/templates
COPY --from=builder /app/migrations /app/migrations

# Create non-root user
RUN useradd -r -s /bin/false oppskrift
USER oppskrift

EXPOSE 3000

ENV RUST_LOG=info
ENV HOST=0.0.0.0
ENV PORT=3000

CMD ["/app/oppskrift"]
