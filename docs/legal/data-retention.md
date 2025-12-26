# Data Retention Policy

This document defines the retention periods for personal data processed by Oppskrift, in compliance with GDPR Article 5(1)(e) (storage limitation).

## Retention Principles

1. **Purpose Limitation**: Data is retained only as long as necessary for its purpose
2. **Minimization**: Expired data is deleted or anonymized
3. **Transparency**: Users are informed of retention periods
4. **Accountability**: Retention decisions are documented

## Retention Schedule

### User Account Data

| Data Type | Retention Period | Justification |
|-----------|------------------|---------------|
| User profile | Until account deletion | Required for service operation |
| Username | Until account deletion | Identity in federated network |
| Email address | Until account deletion | Account recovery, notifications |
| Avatar image | Until account deletion or replacement | Profile display |
| Preferences | Until account deletion | Personalization |
| RSA keys | Until account deletion | Federation authentication |

**Post-Deletion**:
- Account data is deleted within 30 days of deletion request
- Username may be reserved for 90 days to prevent impersonation
- Backup copies retained for 30 days after deletion

### Content Data

| Data Type | Retention Period | Justification |
|-----------|------------------|---------------|
| Recipes | Until deleted by user | User-created content |
| Recipe images | Until deleted or recipe removed | Content display |
| Recipe books | Until deleted by user | Content organization |
| Comments | Until deleted by user or parent deleted | User engagement |

**Post-Deletion**:
- Content is soft-deleted and purged after 30 days
- Federated copies may persist on remote servers
- ActivityPub Delete activities are sent to followers

### Interaction Data

| Data Type | Retention Period | Justification |
|-----------|------------------|---------------|
| Follows | Until unfollowed | Social graph |
| Likes | Until unliked | User preferences |
| Saved recipes | Until unsaved | User bookmarks |
| Activity feed | 1 year | Recent activity display |

### Federation Data

| Data Type | Retention Period | Justification |
|-----------|------------------|---------------|
| Remote actor cache | 7 days | Performance optimization |
| Inbox activities | 90 days | Processing and debugging |
| Outbox activities | 1 year | Activity history |
| HTTP Signature logs | 30 days | Security audit |

### Security & Operational Data

| Data Type | Retention Period | Justification |
|-----------|------------------|---------------|
| Access logs | 90 days | Security monitoring |
| Audit events | 1 year | Compliance, incident investigation |
| Error logs | 30 days | Debugging, reliability |
| Rate limit data | 24 hours | DDoS protection |
| Session tokens | Until logout or 24 hours | Authentication |

### Anonymized Data

| Data Type | Retention Period | Justification |
|-----------|------------------|---------------|
| Aggregated analytics | 2 years | Service improvement |
| Performance metrics | 1 year | Infrastructure planning |

## Deletion Procedures

### User-Initiated Deletion

1. User requests deletion via account settings
2. Account is immediately deactivated
3. Personal data is queued for deletion
4. Deletion completes within 30 days
5. Confirmation email sent (if email provided)

### Automated Cleanup

Daily cleanup job processes:

```sql
-- Delete expired activities
DELETE FROM activities
WHERE created_at < NOW() - INTERVAL '1 year';

-- Delete expired security logs
DELETE FROM audit_logs
WHERE created_at < NOW() - INTERVAL '1 year';

-- Purge soft-deleted content older than 30 days
DELETE FROM recipes
WHERE deleted_at < NOW() - INTERVAL '30 days';
```

### Federation Considerations

When user data is deleted:

1. Local data is immediately removed from public view
2. ActivityPub `Delete` activity is sent to all followers
3. Remote servers should honor the delete request
4. No guarantee of deletion on third-party servers

## Legal Holds

In case of legal proceedings:
- Affected data is exempted from deletion
- Legal team must document the hold
- Data is deleted when hold is lifted
- Maximum hold period: Duration of proceedings + 30 days

## Backup Retention

| Backup Type | Retention | Notes |
|-------------|-----------|-------|
| Daily | 7 days | Rolling window |
| Weekly | 4 weeks | Point-in-time recovery |
| Monthly | 3 months | Disaster recovery |

Backups are encrypted and access-controlled.

## Implementation Status

| Category | Implemented | Notes |
|----------|-------------|-------|
| Account deletion | Yes | Via account settings |
| Content deletion | Yes | Soft delete + purge |
| Audit log cleanup | Planned | T404 (cleanup job) |
| Activity purge | Planned | T404 (cleanup job) |
| Federation delete | Planned | T406 |

## Review Schedule

This policy is reviewed:
- Annually
- When new data types are introduced
- When legal requirements change

---

*Last reviewed: December 2024*
*Next review: December 2025*
