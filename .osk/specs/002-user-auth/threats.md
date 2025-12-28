# Modele de Menaces - User Authentication

> Genere par `/osk-analyze 002-user-auth` le 2025-12-26
> Principes I (Modelisation menaces) et II (Analyse de risques)

## Resume Executif

| Categorie STRIDE | Nombre | Critique | Important | Moyen |
|------------------|--------|----------|-----------|-------|
| **S** - Spoofing | 6 | 2 | 3 | 1 |
| **T** - Tampering | 4 | 1 | 2 | 1 |
| **R** - Repudiation | 3 | 0 | 3 | 0 |
| **I** - Information Disclosure | 5 | 1 | 2 | 2 |
| **D** - Denial of Service | 4 | 1 | 2 | 1 |
| **E** - Elevation of Privilege | 4 | 1 | 2 | 1 |
| **Total** | **26** | **6** | **14** | **6** |

---

## Architecture et Trust Boundaries

```
                              INTERNET
                                  |
                 +----------------+----------------+
                 |                |                |
                 v                v                v
         +------------+   +------------+   +------------+
         |  Browser   |   |   Email    |   | Auth App   |
         |  (Login)   |   |  (Links)   |   |  (TOTP)    |
         +-----+------+   +-----+------+   +-----+------+
               |                |                |
================== TRUST BOUNDARY #1 (TB1) ==================
               |                |                |
               v                v                v
         +--------------------------------------------------+
         |                AXUM APPLICATION                   |
         |  +-------------+  +-------------+  +-----------+  |
         |  | Auth API    |  | Session Mgr |  | TOTP Svc  |  |
         |  | (Register,  |  | (Create,    |  | (Verify,  |  |
         |  |  Login,     |  |  Validate,  |  |  Setup)   |  |
         |  |  Logout)    |  |  Revoke)    |  |           |  |
         |  +------+------+  +------+------+  +-----+-----+  |
         |         |                |               |        |
         |  +------+----------------+---------------+----+   |
         |  |           Password Service                 |   |
         |  | (Hash, Verify, Breach Check, Strength)     |   |
         |  +----------------------+---------------------+   |
         |                         |                         |
         |  +----------------------+---------------------+   |
         |  |           Security Event Logger            |   |
         |  | (Login, Logout, Failed, Password Change)   |   |
         |  +----------------------+---------------------+   |
         +-------------------------+-------------------------+
                                   |
        =============== TRUST BOUNDARY #2 (TB2) ===============
                                   |
         +-------------------------+-------------------------+
         |                    PostgreSQL                     |
         |  +----------+ +----------+ +----------+ +-------+ |
         |  |  users   | | sessions | |  tokens  | | events| |
         |  |(creds,   | |(session  | |(reset,   | |(audit)| |
         |  | 2fa)     | | data)    | | confirm) | |       | |
         |  +----------+ +----------+ +----------+ +-------+ |
         +---------------------------------------------------+
                                   |
        =============== TRUST BOUNDARY #3 (TB3) ===============
                                   |
                           +-------+-------+
                           |  SMTP Server  |
                           | (Email Links) |
                           +---------------+
```

### Trust Boundaries

| ID | Nom | Description | Niveau Risque |
|----|-----|-------------|---------------|
| TB1 | Internet -> Application | Point d'entree public (login, register, reset) | CRITIQUE |
| TB2 | Application -> Database | Stockage credentials et sessions | CRITIQUE |
| TB3 | Application -> SMTP | Envoi emails confirmation/reset (liens sensibles) | IMPORTANT |

---

## Assets Critiques

| Asset | Sensibilite | Impact Compromission |
|-------|-------------|---------------------|
| Mots de passe (hashes) | CRITIQUE | Compromission comptes en cascade |
| Secrets TOTP | CRITIQUE | Bypass 2FA complet |
| Tokens de session | CRITIQUE | Usurpation identite |
| Tokens reset password | IMPORTANT | Prise de controle compte |
| Recovery codes 2FA | IMPORTANT | Bypass 2FA |
| Emails utilisateurs | IMPORTANT | Enumeration, phishing cible |
| Evenements securite | IMPORTANT | Effacement traces attaque |

---

## Menaces Detaillees par Categorie

### S - Spoofing (Usurpation d'identite)

#### THREAT-AUTH-S-001 - Credential Stuffing

| Attribut | Valeur |
|----------|--------|
| Severite | CRITIQUE |
| Probabilite | 5/5 (Attaque automatisee courante) |
| Impact | 5/5 (Compromission comptes) |
| Asset Menace | Comptes utilisateur |

**Description :** Un attaquant utilise des listes de credentials fuites d'autres services pour tenter des connexions automatisees.

**Vecteur d'attaque :**
1. Obtenir listes email/password de breaches publiques
2. Automatiser requetes POST /api/auth/login
3. Identifier credentials valides par code 200

**Controles requis :**
- Rate limiting par IP (10/min) - FR-011
- Rate limiting par compte (5 tentatives puis lockout 15min) - FR-010
- Detection patterns automatises (captcha apres echecs)
- Verification mot de passe contre breaches - FR-004

---

#### THREAT-AUTH-S-002 - Session Hijacking

| Attribut | Valeur |
|----------|--------|
| Severite | CRITIQUE |
| Probabilite | 3/5 (Necessite position reseau ou XSS) |
| Impact | 5/5 (Acces complet compte) |
| Asset Menace | Tokens de session |

**Description :** Vol du token de session via interception reseau (MITM) ou injection XSS.

**Vecteur d'attaque :**
1. Interception token via WiFi public (si pas HTTPS)
2. Ou extraction via XSS stockee
3. Replay du token pour acceder au compte

**Controles requis :**
- HTTPS obligatoire (Assumption validee)
- Cookies HttpOnly, Secure, SameSite=Strict
- Rotation token apres actions sensibles
- Binding IP optionnel avec warning

---

#### THREAT-AUTH-S-003 - Password Reset Token Theft

| Attribut | Valeur |
|----------|--------|
| Severite | IMPORTANT |
| Probabilite | 3/5 (Email interception, acces boite mail) |
| Impact | 5/5 (Prise controle compte) |
| Asset Menace | Tokens reset password |

**Description :** Vol du lien de reinitialisation via acces email compromis ou interception.

**Vecteur d'attaque :**
1. Demander reset pour victime
2. Intercepter email (acces boite mail ou MITM SMTP)
3. Utiliser lien avant la victime

**Controles requis :**
- Tokens single-use - FR-017
- Expiration courte (1h) - FR-016
- Invalidation tokens existants a nouvelle demande
- Log security event sur utilisation

---

#### THREAT-AUTH-S-004 - Email Confirmation Link Abuse

| Attribut | Valeur |
|----------|--------|
| Severite | IMPORTANT |
| Probabilite | 2/5 (Fenetre temporelle limitee) |
| Impact | 4/5 (Usurpation compte non confirme) |
| Asset Menace | Tokens confirmation email |

**Description :** Interception du lien de confirmation pour activer un compte avant l'utilisateur legitime.

**Vecteur d'attaque :**
1. Creer compte avec email cible
2. Intercepter email confirmation
3. Activer compte et prendre controle

**Controles requis :**
- Expiration 24h - FR-007
- Token single-use
- Notification email a confirmation

---

#### THREAT-AUTH-S-005 - TOTP Brute Force

| Attribut | Valeur |
|----------|--------|
| Severite | IMPORTANT |
| Probabilite | 2/5 (Fenetre temporelle TOTP) |
| Impact | 4/5 (Bypass 2FA) |
| Asset Menace | Secret TOTP |

**Description :** Tentative de brute force du code TOTP (1M combinaisons, mais fenetre 30s).

**Vecteur d'attaque :**
1. Credentials valides obtenus
2. Enumeration codes TOTP (000000-999999)
3. Avec fenetre +/-1 = 3 codes valides

**Controles requis :**
- Rate limiting strict sur verification TOTP (3 tentatives/10min)
- Lockout compte apres echecs TOTP repetes
- Logging tentatives TOTP

---

#### THREAT-AUTH-S-006 - Username Enumeration

| Attribut | Valeur |
|----------|--------|
| Severite | MOYEN |
| Probabilite | 4/5 (Endpoints multiples) |
| Impact | 2/5 (Info pour attaque ciblee) |
| Asset Menace | Liste usernames/emails |

**Description :** Differences de reponse permettant d'enumerer les comptes existants.

**Vecteur d'attaque :**
1. Tester registration avec email
2. Observer difference reponse "email existe" vs "email disponible"
3. Ou tester reset password et observer timing

**Controles requis :**
- Messages identiques "email existe" / "email libre" - FR-003
- Messages identiques reset password - FR-018
- Timing constant sur verifications
- Generic error messages login - FR-014

---

### T - Tampering (Alteration)

#### THREAT-AUTH-T-001 - Password Hash Weak Algorithm

| Attribut | Valeur |
|----------|--------|
| Severite | CRITIQUE |
| Probabilite | 2/5 (Necessite acces DB) |
| Impact | 5/5 (Compromission tous mots de passe) |
| Asset Menace | Hashes mots de passe |

**Description :** Utilisation d'un algorithme de hashing faible (MD5, SHA1, bcrypt cout faible) permettant cracking offline.

**Vecteur d'attaque :**
1. Obtenir dump database (SQL injection, backup expose)
2. Extraire table users avec hashes
3. Cracker offline avec GPU

**Controles requis :**
- Argon2id avec parametres OWASP
- Cost factor adapte au hardware
- Pepper secret en plus du salt

---

#### THREAT-AUTH-T-002 - Session Token Prediction

| Attribut | Valeur |
|----------|--------|
| Severite | IMPORTANT |
| Probabilite | 2/5 (CSPRNG moderne) |
| Impact | 5/5 (Session forgery) |
| Asset Menace | Tokens session |

**Description :** Tokens de session generes avec entropie insuffisante, permettant prediction.

**Vecteur d'attaque :**
1. Collecter tokens legitimes
2. Analyser pattern de generation
3. Predire tokens valides

**Controles requis :**
- UUID v4 ou random 256 bits
- CSPRNG (rand crate avec OsRng)
- Stockage hash du token cote serveur

---

#### THREAT-AUTH-T-003 - TOTP Secret Extraction

| Attribut | Valeur |
|----------|--------|
| Severite | IMPORTANT |
| Probabilite | 2/5 (Acces DB requis) |
| Impact | 4/5 (Bypass 2FA tous utilisateurs) |
| Asset Menace | Secrets TOTP |

**Description :** Secrets TOTP stockes en clair dans DB, extractibles via SQL injection ou backup.

**Vecteur d'attaque :**
1. Obtenir acces DB
2. Extraire secrets TOTP
3. Generer codes valides pour tout utilisateur

**Controles requis :**
- Chiffrement secrets TOTP au repos
- Cle de chiffrement externe (env var)
- HSM pour environnements critiques

---

#### THREAT-AUTH-T-004 - Recovery Codes Reuse

| Attribut | Valeur |
|----------|--------|
| Severite | MOYEN |
| Probabilite | 2/5 (Implementation) |
| Impact | 3/5 (Acces si 2FA perdu) |
| Asset Menace | Recovery codes |

**Description :** Recovery codes non invalides apres utilisation, permettant reutilisation.

**Vecteur d'attaque :**
1. Obtenir un recovery code (shoulder surfing, backup expose)
2. L'utiliser pour acceder au compte
3. Le reutiliser plus tard

**Controles requis :**
- Marquer code comme "used" immediatement - FR-033
- Regenerer set complet optionnel
- Notifier par email utilisation

---

### R - Repudiation

#### THREAT-AUTH-R-001 - Missing Login Audit Trail

| Attribut | Valeur |
|----------|--------|
| Severite | IMPORTANT |
| Probabilite | 5/5 (Non implemente) |
| Impact | 3/5 (Investigation impossible) |
| Asset Menace | Historique connexions |

**Description :** Absence de log structure des tentatives de connexion (succes et echecs).

**Impact :**
- Impossible de detecter credential stuffing
- Pas de preuve forensique
- Non-conformite RGPD Art. 30

**Controles requis :**
- Log JSON structure - FR-027
- Champs: timestamp, user_id, IP, user_agent, result, reason
- Retention 90 jours minimum

---

#### THREAT-AUTH-R-002 - Session Activity Untracked

| Attribut | Valeur |
|----------|--------|
| Severite | IMPORTANT |
| Probabilite | 5/5 |
| Impact | 3/5 |
| Asset Menace | Activite sessions |

**Description :** Pas de tracking last_activity sur sessions, impossible de voir sessions suspectes.

**Controles requis :**
- FR-022 : Track device type, browser, last activity, IP
- Affichage dans UI compte - FR-019

---

#### THREAT-AUTH-R-003 - Password Change Not Logged

| Attribut | Valeur |
|----------|--------|
| Severite | IMPORTANT |
| Probabilite | 5/5 |
| Impact | 3/5 |
| Asset Menace | Changements credentials |

**Description :** Changements mot de passe non journalises avec details.

**Controles requis :**
- Log: timestamp, user_id, IP, method (reset/change)
- Email notification au user
- FR-027 couverture

---

### I - Information Disclosure

#### THREAT-AUTH-I-001 - Timing Attack Password Verification

| Attribut | Valeur |
|----------|--------|
| Severite | CRITIQUE |
| Probabilite | 3/5 (Mesure precision requise) |
| Impact | 4/5 (Enumeration + partial password) |
| Asset Menace | Mots de passe |

**Description :** Difference de timing entre user inexistant et password incorrect permet enumeration.

**Vecteur d'attaque :**
1. Mesurer temps reponse login user inexistant
2. Mesurer temps reponse login mauvais password
3. Identifier users valides par timing

**Controles requis :**
- Constant-time comparison pour hash
- Fake hash verification si user inexistant
- Messages generiques - FR-014

---

#### THREAT-AUTH-I-002 - Error Message Information Leak

| Attribut | Valeur |
|----------|--------|
| Severite | IMPORTANT |
| Probabilite | 4/5 |
| Impact | 3/5 |
| Asset Menace | Details implementation |

**Description :** Messages d'erreur revelant details implementation (stack traces, DB errors).

**Controles requis :**
- Generic messages en production
- Log details cote serveur seulement
- Error IDs pour correlation support

---

#### THREAT-AUTH-I-003 - Password Requirements Leak

| Attribut | Valeur |
|----------|--------|
| Severite | MOYEN |
| Probabilite | 5/5 (By design) |
| Impact | 2/5 (Aide brute force) |
| Asset Menace | Politique mots de passe |

**Description :** Affichage des regles password aide attaquant a optimiser dictionnaire.

**Note :** Acceptable trade-off car ameliore UX et securite globale.

**Controles requis :**
- Afficher regles clairement - FR-004
- Mais rate limiting fort compense

---

#### THREAT-AUTH-I-004 - Session Token in URL

| Attribut | Valeur |
|----------|--------|
| Severite | IMPORTANT |
| Probabilite | 2/5 (Implementation) |
| Impact | 4/5 (Leak via referer, logs) |
| Asset Menace | Tokens session |

**Description :** Token de session dans URL au lieu de cookie/header, expose dans logs et referer.

**Controles requis :**
- Tokens dans cookies HttpOnly
- Ou header Authorization
- Jamais dans query params

---

#### THREAT-AUTH-I-005 - Email in Logs

| Attribut | Valeur |
|----------|--------|
| Severite | MOYEN |
| Probabilite | 4/5 |
| Impact | 2/5 |
| Asset Menace | Emails utilisateurs |

**Description :** Emails loggues en clair, accessible a ops et potentiel leak.

**Controles requis :**
- Masquer emails dans logs (u***@example.com)
- Ou hash pour correlation
- RGPD Art. 25

---

### D - Denial of Service

#### THREAT-AUTH-D-001 - Account Lockout Abuse

| Attribut | Valeur |
|----------|--------|
| Severite | CRITIQUE |
| Probabilite | 4/5 (Facile a exploiter) |
| Impact | 3/5 (Denial of service cible) |
| Asset Menace | Disponibilite comptes |

**Description :** Attaquant force lockout de comptes cibles en repetant echecs login.

**Vecteur d'attaque :**
1. Identifier usernames cibles
2. Envoyer 5 tentatives incorrectes
3. Compte locke pour 15 minutes

**Controles requis :**
- Lockout progressif (15min, 30min, 1h)
- CAPTCHA avant lockout complet
- Alerte admin sur lockouts massifs
- Self-unlock via email comme alternative

---

#### THREAT-AUTH-D-002 - Registration Flood

| Attribut | Valeur |
|----------|--------|
| Severite | IMPORTANT |
| Probabilite | 4/5 |
| Impact | 3/5 |
| Asset Menace | Ressources serveur, emails |

**Description :** Creation massive de comptes fictifs consommant ressources et emails.

**Controles requis :**
- Rate limit registration par IP
- CAPTCHA sur registration
- Cleanup comptes non confirmes (24h)

---

#### THREAT-AUTH-D-003 - Password Reset Flood

| Attribut | Valeur |
|----------|--------|
| Severite | IMPORTANT |
| Probabilite | 4/5 |
| Impact | 3/5 (Cout SMTP, spam victime) |
| Asset Menace | Service email, UX victime |

**Description :** Flood de demandes reset password pour un email cible.

**Controles requis :**
- Rate limit reset par email (1/5min)
- Rate limit reset par IP (10/min)
- Invalidation anciens tokens sur nouvelle demande

---

#### THREAT-AUTH-D-004 - TOTP Verification DoS

| Attribut | Valeur |
|----------|--------|
| Severite | MOYEN |
| Probabilite | 3/5 |
| Impact | 2/5 |
| Asset Menace | Verification TOTP |

**Description :** Verification TOTP CPU-intensive si mal implementee.

**Controles requis :**
- Rate limit verifications
- Timeout apres tentatives

---

### E - Elevation of Privilege

#### THREAT-AUTH-E-001 - Privilege Escalation via Session Tampering

| Attribut | Valeur |
|----------|--------|
| Severite | CRITIQUE |
| Probabilite | 2/5 (JWT bien implemente) |
| Impact | 5/5 (Admin access) |
| Asset Menace | Roles utilisateur |

**Description :** Modification claims JWT pour s'attribuer privileges admin.

**Vecteur d'attaque :**
1. Decoder JWT (base64)
2. Modifier claim "role" ou "admin"
3. Resigner si secret faible/connu

**Controles requis :**
- Secret JWT fort (>= 256 bits) - VULN-BASELINE-001
- Validation signature stricte
- Roles en DB, pas seulement JWT
- Double-check privileges cote serveur

---

#### THREAT-AUTH-E-002 - IDOR on Account Operations

| Attribut | Valeur |
|----------|--------|
| Severite | IMPORTANT |
| Probabilite | 3/5 |
| Impact | 4/5 |
| Asset Menace | Comptes autres utilisateurs |

**Description :** Modification user_id dans requetes pour operer sur autres comptes.

**Vecteur d'attaque :**
1. Intercepter requete POST /api/account/password
2. Modifier user_id vers cible
3. Changer password d'un autre compte

**Controles requis :**
- User ID depuis session, jamais depuis input
- Authorization check systematique
- Tests IDOR automatises

---

#### THREAT-AUTH-E-003 - Recovery Code as Backdoor

| Attribut | Valeur |
|----------|--------|
| Severite | IMPORTANT |
| Probabilite | 2/5 |
| Impact | 4/5 |
| Asset Menace | Comptes 2FA |

**Description :** Recovery codes mal proteges servant de backdoor permanente.

**Controles requis :**
- Regeneration obligatoire apres utilisation partielle
- Hash storage (pas plaintext)
- Limite 8-10 codes

---

#### THREAT-AUTH-E-004 - Session Fixation

| Attribut | Valeur |
|----------|--------|
| Severite | MOYEN |
| Probabilite | 2/5 |
| Impact | 4/5 |
| Asset Menace | Sessions pre-auth |

**Description :** Attaquant force victime a utiliser session ID connu.

**Vecteur d'attaque :**
1. Obtenir session ID valide
2. Forcer victime a l'utiliser (lien, XSS)
3. Victime s'authentifie
4. Attaquant accede avec meme session

**Controles requis :**
- Regenerer session ID au login
- Invalider sessions pre-auth
- Binding strict user-session

---

## Arbres d'Attaque - Top 3 Menaces

### Attack Tree 1: Account Takeover via Password Reset

```
[GOAL: Prendre controle compte victime]
    |
    +-- [1. Obtenir token reset] (OR)
    |       |
    |       +-- [1.1 Intercepter email]
    |       |       +-- Acces boite mail victime
    |       |       +-- MITM sur SMTP
    |       |
    |       +-- [1.2 Bruteforce token]
    |       |       +-- Token previsible (weak random)
    |       |       +-- Pas de rate limit sur /reset/:token
    |       |
    |       +-- [1.3 Token dans logs]
    |               +-- Acces logs serveur
    |               +-- Token dans URL loggue
    |
    +-- [2. Utiliser token avant expiration]
            +-- Token single-use? -> BLOCKED
            +-- Expiration 1h? -> Time limited
```

### Attack Tree 2: Bypass 2FA

```
[GOAL: Contourner 2FA]
    |
    +-- [1. Voler secret TOTP] (OR)
    |       +-- SQL injection -> DB dump
    |       +-- Backup non chiffre
    |       +-- Screenshot QR code
    |
    +-- [2. Bruteforce TOTP]
    |       +-- 1M codes / 30s window
    |       +-- Rate limit? -> BLOCKED si 3/10min
    |
    +-- [3. Utiliser Recovery Code]
    |       +-- Phishing code
    |       +-- Shoulder surfing
    |       +-- Backup utilisateur
    |
    +-- [4. Session hijack post-2FA]
    |       +-- XSS stocke
    |       +-- MITM apres login
    |
    +-- [5. Social engineering support]
            +-- Demander reset 2FA
            +-- Faux documents identite
```

### Attack Tree 3: Credential Stuffing at Scale

```
[GOAL: Compromettre comptes via stuffing]
    |
    +-- [1. Preparer attaque]
    |       +-- Obtenir listes email/password
    |       +-- Valider emails existent (enumeration)
    |       +-- Preparer infrastructure distribuee
    |
    +-- [2. Executer stuffing] (AND)
    |       |
    |       +-- [2.1 Evader rate limit IP]
    |       |       +-- Botnet/proxies
    |       |       +-- Rotation IPs
    |       |
    |       +-- [2.2 Evader rate limit compte]
    |               +-- Slow & low (1 attempt/hour/account)
    |               +-- Password non-breach check? -> BLOCKED
    |
    +-- [3. Monetiser acces]
            +-- Exfiltrer donnees
            +-- Pivot vers autres services
```

---

## Matrice de Risques

```
IMPACT
   ^
 5 |  M   H  [H]  C   C     <- Credential stuffing (S-001)
 4 |  L   M   H  [H]  C     <- Session hijack (S-002)
 3 |  L   L   M   H  [H]    <- Account lockout DoS (D-001)
 2 |  L   L   L   M   M
 1 |  L   L   L   L   M
   +----------------------------> PROBABILITE
       1   2   3   4   5

L = Low, M = Medium, H = High, C = Critical
```

---

## Prochaine Etape

-> Voir `risks.md` pour l'analyse detaillee et les plans de mitigation
-> Les controles identifies seront integres dans le plan d'implementation
