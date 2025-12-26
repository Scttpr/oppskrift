# GDPR Legal Basis Documentation

This document details the legal basis for processing personal data in Oppskrift, as required by GDPR Article 6.

## Overview

Oppskrift processes personal data under three primary legal bases:
1. **Contract Performance** (Article 6(1)(b))
2. **Legitimate Interest** (Article 6(1)(f))
3. **Consent** (Article 6(1)(a)) - where applicable

## Data Processing Activities

### 1. User Account Management

| Processing Activity | Data Categories | Legal Basis | Justification |
|---------------------|-----------------|-------------|---------------|
| Account creation | Username, display name, email | Contract | Required to provide the service |
| Profile storage | Bio, avatar, preferences | Contract | User-requested profile features |
| Authentication | Session tokens, password hash | Contract | Required for secure access |

### 2. Recipe & Content Management

| Processing Activity | Data Categories | Legal Basis | Justification |
|---------------------|-----------------|-------------|---------------|
| Recipe storage | Title, ingredients, instructions | Contract | Core service functionality |
| Image storage | Recipe photos, avatars | Contract | User-uploaded content |
| Content indexing | Recipe metadata | Contract | Enable search and discovery |

### 3. Federation (ActivityPub)

| Processing Activity | Data Categories | Legal Basis | Justification |
|---------------------|-----------------|-------------|---------------|
| Public key storage | RSA keypair | Contract | Required for HTTP Signatures |
| Activity distribution | Public content, profile | Contract | Federation is a core feature |
| Remote actor data | Federated user profiles | Legitimate Interest | Enable cross-instance interaction |

**Legitimate Interest Assessment (Federation):**
- Purpose: Enable federated social networking
- Necessity: Required for ActivityPub protocol compliance
- Balancing: Users explicitly choose to federate; public content only
- Safeguards: Users can make content private; instance blocking available

### 4. Security & Operations

| Processing Activity | Data Categories | Legal Basis | Justification |
|---------------------|-----------------|-------------|---------------|
| Access logging | IP, timestamp, user agent | Legitimate Interest | Security, abuse prevention |
| Rate limiting | IP address, request counts | Legitimate Interest | DDoS protection, fair usage |
| Error tracking | Request context, stack traces | Legitimate Interest | Service reliability |

**Legitimate Interest Assessment (Security):**
- Purpose: Protect service and users from attacks
- Necessity: Essential for service operation
- Balancing: Minimal data, short retention (90 days)
- Safeguards: Logs not shared externally; IP anonymization after 30 days

### 5. Communication

| Processing Activity | Data Categories | Legal Basis | Justification |
|---------------------|-----------------|-------------|---------------|
| Service notifications | Email address | Contract | Account-related alerts |
| Marketing emails | Email address | Consent | Only with explicit opt-in |

## Data Retention Periods

| Data Category | Retention Period | Justification |
|---------------|------------------|---------------|
| Account data | Until deletion | User controls lifecycle |
| Content data | Until deletion | User controls lifecycle |
| Security logs | 90 days | Sufficient for incident investigation |
| Anonymized analytics | 2 years | Aggregated, non-personal |
| Backup data | 30 days post-deletion | Disaster recovery |

## Special Categories (Article 9)

Oppskrift does not intentionally collect special category data (health, religion, etc.). However:

- **Dietary preferences** in recipes (vegan, halal, kosher) may indirectly reveal beliefs
- These are user-initiated and published publicly
- Legal basis: Explicit consent through voluntary publication

## Children's Data

- Minimum age: 16 years (GDPR default)
- No verification mechanism currently implemented
- Parental consent required for users under 16

## Data Subject Rights Implementation

| Right | Implementation |
|-------|----------------|
| Access (Art. 15) | Profile page shows all stored data |
| Rectification (Art. 16) | Edit profile/content functionality |
| Erasure (Art. 17) | Account deletion feature |
| Portability (Art. 20) | Export data as JSON (planned) |
| Restriction (Art. 18) | Account suspension feature |
| Object (Art. 21) | Contact administrator |

## Third-Party Data Processing

| Processor | Data Shared | Purpose | Safeguards |
|-----------|-------------|---------|------------|
| Hosting provider | All stored data | Infrastructure | DPA, EU hosting |
| S3 storage | Images | Media storage | DPA, encryption |
| Federated instances | Public content | Federation | ActivityPub protocol |

## Record of Processing Activities (ROPA)

This documentation serves as part of the ROPA required by GDPR Article 30. Full records are maintained by the data controller.

## Review Schedule

This document should be reviewed:
- Annually
- When new features are added
- When processing activities change
- After data protection incidents

---

*Last reviewed: December 2024*
