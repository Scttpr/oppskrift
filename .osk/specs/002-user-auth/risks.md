# Analyse des Risques - User Authentication

> Genere par `/osk-analyze 002-user-auth` le 2025-12-26
> Vue consolidee des risques feature authentication

## Resume Executif

| Metrique | Valeur |
|----------|--------|
| Risques totaux | 12 |
| Score moyen | 62.5 |
| Risques critiques | 3 |
| Risques importants | 6 |
| Risques moyens | 3 |

### Distribution par Severite

```
CRITIQUE     [===]     3 (25%)
IMPORTANT    [======]  6 (50%)
MOYEN        [===]     3 (25%)
MINEUR       []        0 (0%)
```

---

## Risques par Priorite

### P0 - Critique (Implementation obligatoire)

#### RISK-AUTH-001 - Credential Stuffing Non Mitige

| Attribut | Valeur |
|----------|--------|
| Score | 100 (I:5 x P:5 x E:4) |
| STRIDE | S (Spoofing) |
| CWE | CWE-307 |
| OWASP | A07:2021 - Identification and Authentication Failures |
| Menace | THREAT-AUTH-S-001 |

**Description :** Sans rate limiting adequat et verification breaches, credential stuffing permet compromission de masse.

**Controles requis (spec) :**
- FR-010: Rate limiting 5 attempts/account, 15min lockout
- FR-011: Rate limiting 10 attempts/IP/minute
- FR-004: Verification contre liste mots de passe compromis

**Implementation :**
- [ ] Middleware rate limiting avec sliding window
- [ ] Table `login_attempts` avec cleanup automatique
- [ ] Integration API Have I Been Pwned (k-anonymity)
- [ ] Tests de charge credential stuffing

---

#### RISK-AUTH-002 - Password Storage Insecure

| Attribut | Valeur |
|----------|--------|
| Score | 90 (I:5 x P:3 x E:6) |
| STRIDE | T (Tampering) |
| CWE | CWE-916 |
| OWASP | A02:2021 - Cryptographic Failures |
| Menace | THREAT-AUTH-T-001 |

**Description :** Algorithme de hashing faible ou mal configure permet cracking offline si DB compromise.

**Controles requis (spec) :**
- FR-009: Algorithme securise moderne (pas plaintext ni weak)

**Implementation :**
- [ ] Argon2id avec OWASP recommended params
- [ ] m=19456 (19 MiB), t=2, p=1 minimum
- [ ] Migration hashes existants au login
- [ ] Pepper applicatif (env var)

---

#### RISK-AUTH-003 - Session Hijacking via XSS/MITM

| Attribut | Valeur |
|----------|--------|
| Score | 75 (I:5 x P:3 x E:5) |
| STRIDE | S (Spoofing) |
| CWE | CWE-384, CWE-614 |
| OWASP | A07:2021 |
| Menace | THREAT-AUTH-S-002 |

**Description :** Tokens de session non proteges contre vol via XSS ou interception.

**Controles requis (spec) :**
- FR-012: Sessions securisees avec expiration configurable (7 jours)
- Assumption: HTTPS configure

**Implementation :**
- [ ] Cookie flags: HttpOnly, Secure, SameSite=Strict
- [ ] Token storage: hash en DB, full token jamais logged
- [ ] Session rotation apres login et actions sensibles
- [ ] Binding user-agent optionnel avec warning

---

### P1 - Important (Implementation recommandee)

#### RISK-AUTH-004 - Account Lockout DoS

| Attribut | Valeur |
|----------|--------|
| Score | 60 (I:3 x P:4 x E:5) |
| STRIDE | D (Denial of Service) |
| CWE | CWE-645 |
| OWASP | A07:2021 |
| Menace | THREAT-AUTH-D-001 |

**Description :** Attaquant peut intentionnellement verrouiller des comptes cibles.

**Controles requis (spec) :**
- FR-010: Lockout 15 minutes apres 5 echecs

**Implementation :**
- [ ] Lockout progressif: 15min -> 30min -> 1h
- [ ] Option CAPTCHA avant lockout definitif
- [ ] Self-unlock via email comme alternative
- [ ] Monitoring alertes lockouts anormaux

---

#### RISK-AUTH-005 - Missing Audit Trail

| Attribut | Valeur |
|----------|--------|
| Score | 60 (I:3 x P:5 x E:4) |
| STRIDE | R (Repudiation) |
| CWE | CWE-778 |
| OWASP | A09:2021 - Security Logging and Monitoring Failures |
| Menaces | THREAT-AUTH-R-001, R-002, R-003 |

**Description :** Absence de logging structure empeche detection intrusion et investigation.

**Controles requis (spec) :**
- FR-027: Log tous evenements securite

**Implementation :**
- [ ] Table `security_events` avec retention
- [ ] Events: login_success, login_failed, logout, password_change, 2fa_enable, session_revoke
- [ ] Champs: timestamp, user_id, ip, user_agent, event_type, metadata (JSON)
- [ ] Export JSON pour SIEM

---

#### RISK-AUTH-006 - Password Reset Token Vulnerabilities

| Attribut | Valeur |
|----------|--------|
| Score | 60 (I:5 x P:3 x E:4) |
| STRIDE | S (Spoofing) |
| CWE | CWE-640 |
| OWASP | A07:2021 |
| Menace | THREAT-AUTH-S-003 |

**Description :** Tokens de reset password mal generes ou non securises.

**Controles requis (spec) :**
- FR-016: Expiration 1 heure
- FR-017: Single-use
- FR-018: Pas d'enumeration email

**Implementation :**
- [ ] Token: 256-bit random (hex string 64 chars)
- [ ] Stockage: hash bcrypt du token en DB
- [ ] Invalidation: anciens tokens a nouvelle demande
- [ ] Rate limit: 1 demande / 5 min / email

---

#### RISK-AUTH-007 - TOTP Implementation Weaknesses

| Attribut | Valeur |
|----------|--------|
| Score | 48 (I:4 x P:3 x E:4) |
| STRIDE | S, T (Spoofing, Tampering) |
| CWE | CWE-308 |
| OWASP | A07:2021 |
| Menaces | THREAT-AUTH-S-005, T-003 |

**Description :** Implementation 2FA avec faiblesses (secret en clair, brute force possible).

**Controles requis (spec) :**
- FR-028 a FR-033: Setup, verification, recovery codes

**Implementation :**
- [ ] Secret TOTP chiffre au repos (AES-256-GCM)
- [ ] Rate limit: 3 tentatives TOTP / 10 min
- [ ] Recovery codes: hash bcrypt, 8 codes, single-use
- [ ] Notification email activation/desactivation 2FA

---

#### RISK-AUTH-008 - Username/Email Enumeration

| Attribut | Valeur |
|----------|--------|
| Score | 40 (I:2 x P:4 x E:5) |
| STRIDE | I (Information Disclosure) |
| CWE | CWE-204 |
| OWASP | A07:2021 |
| Menace | THREAT-AUTH-S-006 |

**Description :** Differences reponses permettent enumerer comptes.

**Controles requis (spec) :**
- FR-003: Pas de revelation email existe (registration)
- FR-014: Erreur generique login
- FR-018: Pas d'enumeration reset

**Implementation :**
- [ ] Registration: "Si email non enregistre, verification envoyee"
- [ ] Login: "Invalid credentials" (jamais "user not found")
- [ ] Reset: "Si email enregistre, instructions envoyees"
- [ ] Timing constant via fake hash check

---

#### RISK-AUTH-009 - Session Management Gaps

| Attribut | Valeur |
|----------|--------|
| Score | 45 (I:3 x P:3 x E:5) |
| STRIDE | I, E (Info Disclosure, Elevation) |
| CWE | CWE-613 |
| OWASP | A07:2021 |
| Menaces | THREAT-AUTH-E-002, I-004 |

**Description :** Gestion sessions incomplete (pas de revocation, pas de tracking).

**Controles requis (spec) :**
- FR-019, FR-020, FR-021, FR-022: Vue, revocation, logout, metadata

**Implementation :**
- [ ] Table `sessions` avec metadata complete
- [ ] Endpoint GET /api/account/sessions
- [ ] Endpoint DELETE /api/account/sessions/:id
- [ ] Update `last_activity` periodique

---

### P2 - Moyen (Implementation souhaitee)

#### RISK-AUTH-010 - Timing Attack Vectors

| Attribut | Valeur |
|----------|--------|
| Score | 36 (I:4 x P:3 x E:3) |
| STRIDE | I (Information Disclosure) |
| CWE | CWE-208 |
| OWASP | A02:2021 |
| Menace | THREAT-AUTH-I-001 |

**Description :** Timing differences revelent existence comptes.

**Implementation :**
- [ ] Fake hash verification si user inexistant
- [ ] Constant-time string comparison
- [ ] Jitter aleatoire sur reponses (optionnel)

---

#### RISK-AUTH-011 - Session Fixation

| Attribut | Valeur |
|----------|--------|
| Score | 32 (I:4 x P:2 x E:4) |
| STRIDE | E (Elevation of Privilege) |
| CWE | CWE-384 |
| OWASP | A07:2021 |
| Menace | THREAT-AUTH-E-004 |

**Description :** Attaquant force utilisation session ID connu.

**Implementation :**
- [ ] Regenerer session ID au login (obligatoire)
- [ ] Invalider toute session pre-auth
- [ ] Cookie SameSite=Strict

---

#### RISK-AUTH-012 - Recovery Codes Abuse

| Attribut | Valeur |
|----------|--------|
| Score | 28 (I:4 x P:2 x E:3.5) |
| STRIDE | E (Elevation of Privilege) |
| CWE | CWE-640 |
| OWASP | A07:2021 |
| Menace | THREAT-AUTH-E-003 |

**Description :** Recovery codes mal proteges deviennent backdoor.

**Implementation :**
- [ ] 8 codes, bcrypt hashes
- [ ] Mark used immediatement
- [ ] Email notification utilisation
- [ ] Regeneration complete optionnelle

---

## Conformite Requirements Spec

| Requirement | Risque | Controle | Status |
|-------------|--------|----------|--------|
| FR-003 | AUTH-008 | Generic registration error | TODO |
| FR-004 | AUTH-001 | Breach password check | TODO |
| FR-009 | AUTH-002 | Secure hashing | TODO |
| FR-010 | AUTH-001, AUTH-004 | Account rate limit | TODO |
| FR-011 | AUTH-001 | IP rate limit | TODO |
| FR-012 | AUTH-003 | Secure sessions | TODO |
| FR-013 | AUTH-009 | Invalidate on password change | TODO |
| FR-014 | AUTH-008 | Generic login error | TODO |
| FR-016 | AUTH-006 | Reset expiration 1h | TODO |
| FR-017 | AUTH-006 | Single-use reset | TODO |
| FR-018 | AUTH-008 | No email enumeration reset | TODO |
| FR-019-022 | AUTH-009 | Session management | TODO |
| FR-027 | AUTH-005 | Security event logging | TODO |
| FR-028-033 | AUTH-007, AUTH-012 | TOTP 2FA | TODO |

---

## Metriques Cibles

| Metrique | Cible | Mesure |
|----------|-------|--------|
| Credential stuffing blocked | 100% detecte | Rate limit + breach check |
| Password hash strength | Argon2id OWASP | Config validation |
| Session security | A+ | Cookie flags, rotation |
| Audit coverage | 100% events | Security event table |
| 2FA adoption | > 20% users | Dashboard tracking |

---

## Actions Prioritaires

1. **Immediate (P0)**
   - Implementer Argon2id pour password hashing
   - Rate limiting avec sliding window
   - Session cookies securises

2. **Court terme (P1)**
   - Audit logging complet
   - TOTP avec encryption
   - Password reset securise

3. **Moyen terme (P2)**
   - Timing attack mitigation
   - Session fixation prevention
   - Recovery codes hardening

---

## Prochaine Etape

-> Integrer ces controles dans `plan.md`
-> Generer `tasks.md` via `/speckit.tasks`
-> Implementer Phase 0 (P0 critiques) en premier
