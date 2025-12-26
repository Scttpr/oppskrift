# Secret Rotation Procedures

This document outlines the procedures for rotating secrets used by Oppskrift.

## Secrets Overview

| Secret | Location | Rotation Frequency | Impact |
|--------|----------|-------------------|--------|
| JWT_SECRET | Environment | 90 days | All user sessions invalidated |
| DATABASE_URL | Environment | On compromise | Requires DB credential update |
| S3 Credentials | Environment | 90 days | Brief upload disruption |
| RSA User Keys | Database | Annual or on compromise | Federation signatures |

## JWT_SECRET Rotation

The JWT secret is used to sign authentication tokens.

### Procedure

1. **Generate new secret**:
   ```bash
   openssl rand -base64 48
   ```

2. **Prepare for rotation**:
   - Schedule maintenance window
   - Notify users of session reset

3. **Update environment**:
   ```bash
   # Update .env or secrets manager
   JWT_SECRET=<new-secret>
   ```

4. **Restart application**:
   ```bash
   systemctl restart oppskrift
   ```

5. **Verify**:
   - Check logs for startup success
   - Test login functionality
   - Confirm old tokens are rejected

### Rollback

If issues occur:
1. Revert JWT_SECRET to previous value
2. Restart application
3. Investigate before retrying

## Database Credentials Rotation

### Procedure

1. **Create new database user**:
   ```sql
   CREATE USER oppskrift_new WITH PASSWORD 'new-secure-password';
   GRANT ALL PRIVILEGES ON DATABASE oppskrift TO oppskrift_new;
   GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA public TO oppskrift_new;
   ```

2. **Update connection string**:
   ```bash
   DATABASE_URL=postgres://oppskrift_new:new-secure-password@host/oppskrift
   ```

3. **Restart application**

4. **Verify connections**

5. **Drop old user** (after confirmation):
   ```sql
   DROP USER oppskrift_old;
   ```

## S3 Credentials Rotation

### AWS IAM (Recommended)

1. Create new access key in AWS Console
2. Update environment:
   ```bash
   AWS_ACCESS_KEY_ID=<new-key>
   AWS_SECRET_ACCESS_KEY=<new-secret>
   ```
3. Restart application
4. Verify uploads work
5. Deactivate old access key in AWS Console
6. Delete old access key after 24 hours

### MinIO / Self-hosted

1. Create new credentials in MinIO console
2. Update environment variables
3. Restart and verify
4. Remove old credentials

## RSA User Keys Rotation

User RSA keys are used for ActivityPub HTTP Signatures.

### Individual User Key Rotation

1. **Generate new keypair** (automatic on next request):
   ```sql
   UPDATE user_keys
   SET
     public_key_pem = '<new-public-key>',
     private_key_pem = '<new-private-key>',
     rotated_at = NOW()
   WHERE user_id = '<user-uuid>';
   ```

2. **Federation impact**:
   - Remote servers may cache old key
   - Allow 24-48 hours for propagation
   - Some remote follows may fail temporarily

### Emergency Key Rotation (All Users)

Only in case of key compromise:

1. Stop federation jobs
2. Regenerate all keys:
   ```sql
   -- Mark all keys for rotation
   UPDATE user_keys SET rotated_at = NULL;
   ```
3. Trigger key regeneration via application
4. Resume federation

## Secrets Management Best Practices

1. **Never commit secrets** to version control
2. **Use secrets manager** (HashiCorp Vault, AWS Secrets Manager) in production
3. **Audit access** to secrets regularly
4. **Document rotations** in operations log
5. **Test rotation procedures** in staging first

## Emergency Procedures

### Suspected Compromise

1. **Immediate Actions**:
   - Rotate affected secret immediately
   - Preserve logs for investigation
   - Check for unauthorized access

2. **Investigation**:
   - Review audit logs
   - Check for data exfiltration
   - Identify compromise vector

3. **Recovery**:
   - Rotate all related credentials
   - Notify affected users if required
   - Document incident

## Rotation Schedule

| Quarter | Actions |
|---------|---------|
| Q1 | JWT_SECRET, S3 credentials |
| Q2 | Review and update procedures |
| Q3 | JWT_SECRET, S3 credentials |
| Q4 | Annual RSA key review, full audit |

---

*Last reviewed: December 2024*
