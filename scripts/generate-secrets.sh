#!/bin/bash
# Generate cryptographically secure secrets for Oppskrift
# Usage: ./scripts/generate-secrets.sh > .env.secrets
#
# SECURITY WARNING:
# - Never commit the output of this script
# - Store secrets securely (vault, encrypted storage)
# - Rotate secrets according to policy in hardening.md

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${YELLOW}# Oppskrift Auth Secrets${NC}"
echo "# Generated: $(date -Iseconds)"
echo "# DO NOT COMMIT THIS FILE"
echo ""

# Check for openssl
if ! command -v openssl &> /dev/null; then
    echo -e "${RED}Error: openssl is required but not installed${NC}" >&2
    exit 1
fi

echo -e "${GREEN}# Authentication Secrets${NC}"
echo ""

# JWT_SECRET - 256-bit (32 bytes) minimum, base64 encoded for safety
echo "# JWT signing key (HMAC-SHA256) - Rotate every 90 days"
echo "JWT_SECRET=$(openssl rand -base64 48 | tr -d '\n')"
echo ""

# TOTP_ENCRYPTION_KEY - exactly 256-bit (32 bytes) hex-encoded
echo "# TOTP secret encryption key (AES-256-GCM) - Rotate annually"
echo "TOTP_ENCRYPTION_KEY=$(openssl rand -hex 32)"
echo ""

echo -e "${GREEN}# Database${NC}"
echo ""
echo "# Generate a strong database password"
echo "# DB_PASSWORD=$(openssl rand -base64 24 | tr -d '\n')"
echo "# DATABASE_URL=postgres://oppskrift:\${DB_PASSWORD}@localhost:5432/oppskrift"
echo ""

echo -e "${GREEN}# Session${NC}"
echo ""
echo "# Example session token (generated per-login, shown for reference)"
echo "# SESSION_TOKEN=$(openssl rand -hex 32)"
echo ""

echo -e "${YELLOW}# Copy the above values to your .env file${NC}"
echo -e "${YELLOW}# Remember to also configure SMTP settings for email delivery${NC}"
