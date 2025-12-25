# Deployment Guide

## Prerequisites

- Docker and Docker Compose
- PostgreSQL 15+
- S3-compatible storage (AWS S3, MinIO, etc.)

## Environment Variables

```bash
# Database
DATABASE_URL=postgres://user:password@localhost:5432/oppskrift

# Application
HOST=0.0.0.0
PORT=3000
BASE_URL=https://your-domain.com

# S3 Storage
S3_BUCKET=oppskrift-images
S3_REGION=us-east-1
S3_ENDPOINT=https://s3.amazonaws.com
AWS_ACCESS_KEY_ID=your-access-key
AWS_SECRET_ACCESS_KEY=your-secret-key

# JWT (for future auth)
JWT_SECRET=your-jwt-secret
```

## Security Requirements

### Encryption at Rest

#### PostgreSQL

PostgreSQL data encryption must be configured at the storage level:

1. **AWS RDS**: Enable encryption at instance creation
   ```
   aws rds create-db-instance \
     --storage-encrypted \
     --kms-key-id alias/your-key
   ```

2. **Self-hosted**: Use encrypted filesystem (LUKS, dm-crypt)
   ```bash
   cryptsetup luksFormat /dev/sdX
   cryptsetup open /dev/sdX pgdata
   mkfs.ext4 /dev/mapper/pgdata
   mount /dev/mapper/pgdata /var/lib/postgresql/data
   ```

3. **Docker**: Mount encrypted volume
   ```yaml
   volumes:
     postgres_data:
       driver: local
       driver_opts:
         type: none
         device: /encrypted/postgres
         o: bind
   ```

#### S3 Storage

Enable server-side encryption for all buckets:

1. **AWS S3**: Enable default encryption
   ```bash
   aws s3api put-bucket-encryption \
     --bucket oppskrift-images \
     --server-side-encryption-configuration '{
       "Rules": [{
         "ApplyServerSideEncryptionByDefault": {
           "SSEAlgorithm": "aws:kms",
           "KMSMasterKeyID": "alias/your-key"
         }
       }]
     }'
   ```

2. **MinIO**: Enable auto-encryption
   ```bash
   mc admin config set myminio/ storage_class standard=EC:0
   mc encrypt set sse-s3 myminio/oppskrift-images
   ```

### Encryption in Transit

- All traffic must use TLS 1.2+
- Configure reverse proxy (nginx/caddy) with valid certificates
- Enable HSTS headers

### Access Control

- Use least-privilege database users
- Rotate credentials regularly
- Enable audit logging

## Deployment Steps

### Docker Compose (Development)

```bash
docker-compose up -d
```

### Production

1. Build the release binary:
   ```bash
   cargo build --release
   ```

2. Run migrations:
   ```bash
   sqlx migrate run
   ```

3. Start the server:
   ```bash
   ./target/release/oppskrift
   ```

## Monitoring

- Health check: `GET /health`
- Metrics: Configure tracing export to your observability stack
