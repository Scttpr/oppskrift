# Analyse des Risques - Baseline

> Genere par `/osk-analyze baseline` le 2025-12-26
> Vue consolidee du registre de risques

## Resume Executif

| Metrique | Valeur |
|----------|--------|
| Risques totaux | 5 |
| Score moyen | 73.8 |
| Risques critiques | 3 |
| Risques importants | 2 |
| Conformite globale | 36% |

### Distribution par Severite

```
CRITIQUE     [===]  3 (60%)
IMPORTANT    [==]   2 (40%)
MOYEN        []     0 (0%)
MINEUR       []     0 (0%)
```

---

## Risques par Priorite

### P0 - Critique (SLA 48h)

#### VULN-BASELINE-001 - JWT Secret Fallback

| Attribut | Valeur |
|----------|--------|
| Score | 100 (I:5 x P:5 x E:4) |
| STRIDE | S (Spoofing) |
| CWE | CWE-798 |
| OWASP | A07:2021 |
| Principe viole | V (Secrets) |

**Fichier :** `src/api/middleware/auth.rs:87`

**Code vulnerable :**
```rust
let secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "dev-secret".to_string());
```

**Controles requis :**
1. Panic! si JWT_SECRET non defini
2. Validation longueur minimale (32 chars)
3. Documentation dans .env.example

**Plan de mitigation :**
- [ ] FIX-001 : Remplacer `unwrap_or_else` par `expect()` (5min)
- [ ] FIX-002 : Creer module Config avec validation (1h)
- [ ] FIX-003 : Integrer Config dans main.rs (30min)

---

#### VULN-BASELINE-002 - Cle Privee Placeholder ActivityPub

| Attribut | Valeur |
|----------|--------|
| Score | 80 (I:4 x P:5 x E:4) |
| STRIDE | S (Spoofing) |
| CWE | CWE-321 |
| OWASP | A02:2021 |
| Principe viole | V (Secrets) |

**Fichier :** `src/jobs/federation.rs:127`

**Code vulnerable :**
```rust
"placeholder_private_key".to_string(), // TODO: Get from DB
```

**Controles requis :**
1. Generer paire RSA par utilisateur
2. Stocker cle privee chiffree en DB
3. Recuperer vraie cle lors de signature

**Plan de mitigation :**
- [ ] FIX-006 : Creer migration `user_keys` (1h)
- [ ] FIX-007 : Generer cles RSA a creation user (2h)

---

#### VULN-BASELINE-003 - Signature HTTP Non Verifiee

| Attribut | Valeur |
|----------|--------|
| Score | 80 (I:4 x P:5 x E:4) |
| STRIDE | S, T (Spoofing, Tampering) |
| CWE | CWE-347 |
| OWASP | A02:2021 |
| Principe viole | III (Exigences securite) |

**Fichiers :**
- `src/api/activitypub.rs:80-81`
- `src/api/activitypub.rs:95-96`

**Code vulnerable :**
```rust
// TODO: Verify HTTP signature from headers
// let signature = headers.get("signature");
```

**Controles requis :**
1. Parser header Signature entrant
2. Fetch cle publique de l'acteur distant
3. Valider signature avant traitement
4. Rejeter avec 401 si invalide

**Plan de mitigation :**
- [ ] Implementer module HTTP Signatures (4h)
- [ ] Integrer dans handlers inbox (2h)
- [ ] Ajouter tests de securite (2h)

---

### P1 - Important (SLA 7 jours)

#### VULN-BASELINE-004 - Cle Publique Placeholder

| Attribut | Valeur |
|----------|--------|
| Score | 64 (I:4 x P:4 x E:4) |
| STRIDE | I (Information Disclosure) |
| CWE | CWE-321 |
| OWASP | A02:2021 |
| Principe viole | V (Secrets) |

**Fichier :** `src/api/activitypub.rs:61`

**Code vulnerable :**
```rust
let public_key_pem = "-----BEGIN PUBLIC KEY-----\nPLACEHOLDER\n-----END PUBLIC KEY-----";
```

**Controles requis :**
1. Recuperer vraie cle publique de DB
2. La retourner dans le profil Actor

**Plan de mitigation :**
- Lie a FIX-006/FIX-007 (generation cles)

---

#### RISK-SYS-001 - Absence d'Audit Logging

| Attribut | Valeur |
|----------|--------|
| Score | 45 (I:3 x P:5 x E:3) |
| STRIDE | R (Repudiation) |
| CWE | CWE-778 |
| OWASP | A09:2021 |
| Principe viole | VI (Audit) |

**Fichiers :** `src/main.rs`, `src/services/**/*.rs`

**Controles requis :**
1. Audit logging JSON structure
2. Logger actions sur donnees personnelles
3. Centralisation des logs

**Plan de mitigation :**
- [ ] FIX-004 : Creer module audit logging (2h)
- [ ] FIX-009 : JSON logging en production (1h)
- [ ] FIX-010 : Ajouter events aux services (4h)

---

## Conformite Principes

| Principe | Score | Statut | Actions Requises |
|----------|-------|--------|------------------|
| I. Modelisation menaces | 30% | CRITIQUE | Analyse STRIDE par feature |
| II. Analyse risques | 30% | CRITIQUE | Scoring et registre |
| III. Exigences securite | 45% | A RISQUE | Verification signatures |
| IV. Tests securite | 40% | A RISQUE | Fuzzing, SAST |
| V. Gestion secrets | 20% | CRITIQUE | Config validation, RSA |
| VI. Audit logging | 25% | CRITIQUE | Audit events |
| VII. Patch management | 60% | ACCEPTABLE | Dependabot config |

### Evolution Cible

```
Actuel:     [====                ] 36%
Phase 0:    [=========           ] 45%
Phase 1:    [===========         ] 55%
Phase 2:    [==============      ] 70%
Phase 3:    [================    ] 80%
```

---

## Risques Residuels Acceptes

Aucun risque accepte a ce stade. Tous les risques identifies doivent etre traites.

---

## Risques Emergents a Surveiller

| ID | Description | Probabilite | Declencheur |
|----|-------------|-------------|-------------|
| EMG-001 | Nouvelles CVE dependencies Rust | Moyenne | cargo audit |
| EMG-002 | Attaques ciblees Fediverse | Faible | Croissance instance |
| EMG-003 | Evolution RGPD | Faible | Nouvelles guidelines CNIL |

---

## Metriques de Suivi

### KPI Securite

| Metrique | Baseline | Cible S+1 | Cible S+4 |
|----------|----------|-----------|-----------|
| Vulns Critiques | 3 | 0 | 0 |
| Vulns Importantes | 2 | 0 | 0 |
| Score Global | 36% | 55% | 80% |
| Couverture audit | 0% | 50% | 100% |

### Commandes de Suivi

```bash
# Dashboard de progression
/osk-dashboard

# Statut des vulnerabilites
/osk-risks

# Executer corrections
/osk-harden
/osk-implement baseline
```

---

## Prochaine Etape

-> `/osk-harden` pour generer les taches de correction detaillees
-> `/osk-implement baseline` pour executer les corrections Phase 0
