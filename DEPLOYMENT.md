# Deployment Guide

## Quick Start (Development)

```bash
# Start all services with Docker/Podman
make up

# App available at http://localhost:3000
```

## Prerequisites

- Docker or Podman with Compose
- PostgreSQL 15+ (or use containerized)
- S3-compatible storage (MinIO included for dev)

## Environment Variables

Copy `.env.example` to `.env` and configure:

```bash
# Required
DATABASE_URL=postgres://oppskrift:oppskrift@localhost:5432/oppskrift
JWT_SECRET=your-secret-minimum-32-characters
S3_BUCKET=oppskrift

# Optional (have defaults)
S3_ENDPOINT=http://localhost:9000
S3_ACCESS_KEY_ID=minioadmin
S3_SECRET_ACCESS_KEY=minioadmin
S3_REGION=us-east-1

HOST=0.0.0.0
PORT=3000
BASE_URL=http://localhost:3000
RUST_LOG=info,oppskrift=debug

# Federation
INSTANCE_DOMAIN=localhost:3000
INSTANCE_NAME=Oppskrift Dev
```

## Docker Compose

The included `docker-compose.yml` is hardened but dev-friendly:

```bash
# Start all services
make up

# Rebuild from scratch
make rebuild

# Stop everything
make down

# View logs
podman logs oppskrift_app_1
```

### Services

| Service | Port | Description |
|---------|------|-------------|
| app | 3000 | Oppskrift application |
| db | 5432 | PostgreSQL database |
| minio | 9000/9001 | S3-compatible storage |

### Security Features

- `no-new-privileges` - Prevent privilege escalation
- `cap_drop: ALL` - Drop all capabilities
- `read_only: true` - Read-only root filesystem
- Internal network for backend services
- Localhost-only port bindings

## Production Deployment

### 1. Build Release Binary

```bash
cargo build --release
```

### 2. Database Setup

```bash
# Run migrations
sqlx migrate run

# Or with SQLx CLI
sqlx database create
sqlx migrate run
```

### 3. Reverse Proxy

Configure nginx/Caddy with TLS:

```nginx
server {
    listen 443 ssl http2;
    server_name your-domain.com;

    ssl_certificate /path/to/cert.pem;
    ssl_certificate_key /path/to/key.pem;

    location / {
        proxy_pass http://127.0.0.1:3000;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

### 4. Run Application

```bash
# Direct
./target/release/oppskrift

# Or with systemd
sudo systemctl start oppskrift
```

## Security Requirements

### Encryption at Rest

#### PostgreSQL

- **AWS RDS**: Enable encryption at instance creation
- **Self-hosted**: Use LUKS/dm-crypt encrypted filesystem
- **Docker**: Mount encrypted volume

#### S3 Storage

- **AWS S3**: Enable default bucket encryption (SSE-S3 or SSE-KMS)
- **MinIO**: Enable auto-encryption with `mc encrypt set`

### Encryption in Transit

- All traffic must use TLS 1.2+
- Enable HSTS headers
- Use valid certificates (Let's Encrypt)

### Access Control

- Use least-privilege database users
- Rotate credentials regularly
- Enable audit logging

## Health Checks

```bash
# Check app responds
curl http://localhost:3000/recipes

# Check database
podman exec oppskrift_db_1 pg_isready -U oppskrift
```

## Troubleshooting

### Container won't start

```bash
# Check logs
podman logs oppskrift_app_1

# Common issues:
# - JWT_SECRET too short (needs 32+ chars)
# - Database not ready (wait for healthy status)
# - Port already in use
```

### Database connection failed

```bash
# Verify database is running
make db

# Check connectivity
psql $DATABASE_URL -c "SELECT 1"
```

### Podman "container already exists"

```bash
# Clean up and restart
make down
podman system prune -f
make up
```
