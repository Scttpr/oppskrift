# Analyse STRIDE Système

> Généré par `/osk-baseline` le 2025-12-26
> Analyse de haut niveau du système global

## Architecture

| Attribut | Valeur |
|----------|--------|
| Type | Monolithe modulaire |
| Exposition | Internet public (API + ActivityPub) |
| Composants | 5 |

### Composants principaux

- **Axum API** (Rust/Axum) - Exposition: Internet
- **PostgreSQL** (PostgreSQL 15) - Exposition: Interne
- **MinIO** (S3-compatible) - Exposition: Interne + CDN
- **Fediverse** (ActivityPub) - Exposition: Internet fédéré
- **Background Jobs** (Tokio) - Exposition: Interne

## Diagramme de Flux (DFD)

```
                                 INTERNET
                                     │
                    ┌────────────────┼────────────────┐
                    │                │                │
                    ▼                ▼                ▼
            ┌───────────┐    ┌───────────┐    ┌───────────┐
            │  Browser  │    │ Fediverse │    │ RSS Reader│
            │  (User)   │    │ (Servers) │    │  (Anon)   │
            └─────┬─────┘    └─────┬─────┘    └─────┬─────┘
                  │                │                │
   ══════════════════════════ TRUST BOUNDARY #1 ══════════════════
                  │                │                │
                  ▼                ▼                ▼
            ┌─────────────────────────────────────────────┐
            │              AXUM APPLICATION               │
            │  ┌─────────┐  ┌─────────┐  ┌─────────┐     │
            │  │REST API │  │ActivityPub│  │  Feeds  │     │
            │  │ (JWT)   │  │ (HTTP Sig)│  │(Public) │     │
            │  └────┬────┘  └────┬────┘  └────┬────┘     │
            │       │            │            │          │
            │       └────────────┼────────────┘          │
            │                    │                        │
            │            ┌───────▼───────┐               │
            │            │   Services    │               │
            │            └───────┬───────┘               │
            └────────────────────┼────────────────────────┘
                                 │
        ═══════════════ TRUST BOUNDARY #2 ═══════════════
                                 │
            ┌────────────────────┼────────────────────┐
            │                    │                    │
            ▼                    ▼                    ▼
     ┌────────────┐       ┌────────────┐       ┌────────────┐
     │ PostgreSQL │       │   MinIO    │       │  Fediverse │
     │  (Data)    │       │  (Files)   │       │ (Federated)│
     └────────────┘       └────────────┘       └────────────┘
           │                    │                    │
    ═══ TRUST #2 ═══     ═══ TRUST #3 ═══    ═══ TRUST #4 ═══
```

## Trust Boundaries

| ID | Nom | Risque |
|----|-----|--------|
| TB1 | Internet → Application | Critique (exposition publique) |
| TB2 | Application → Database | Important (données sensibles) |
| TB3 | Application → Storage | Moyen (fichiers utilisateur) |
| TB4 | Application ↔ Fediverse | Critique (fédération non vérifiée) |

## Menaces STRIDE Système

### Spoofing (Usurpation)

- Usurpation JWT avec secret par défaut "dev-secret"
- Usurpation d'identité ActivityPub (signatures non vérifiées)
- Usurpation d'acteur fédéré (inbox non protégée)

### Tampering (Altération)

- Modification des activités en transit (pas de vérification signature)
- Altération des clés publiques (placeholder hardcodé)
- Injection via inbox ActivityPub (validation minimale)

### Repudiation (Répudiation)

- Actions utilisateur non traçables (pas d'audit logging)
- Activités fédérées non journalisées
- Pas de preuve cryptographique des actions

### Information Disclosure (Divulgation)

- Recettes privées potentiellement exposées via ActivityPub
- Profils utilisateurs accessibles sans authentification
- Erreurs internes potentiellement verboses

### Denial of Service (Déni de service)

- Rate limiting 10 req/s peut être insuffisant
- Pas de protection DDoS niveau infrastructure
- Inbox ActivityPub sans rate limiting spécifique

### Elevation of Privilege (Élévation)

- IDOR possible sur saved recipes (vérifié mais à auditer)
- Pas de séparation admin/user formelle
- Vérification owner_id manuelle (risque d'oubli)

## Risques Système Identifiés

### RISK-SYS-001 - JWT Secret Fallback Production

| Attribut | Valeur |
|----------|--------|
| STRIDE | S (Spoofing) |
| Score | 100 (I:5 × P:5 × E:4) |
| Sévérité | CRITIQUE |

Le code utilise `unwrap_or_else(|_| "dev-secret")` permettant à une instance mal configurée d'utiliser un secret prévisible en production.

**Contrôles requis :**
- Panic! si JWT_SECRET non défini en production
- Validation longueur minimale du secret (32 chars)
- Documentation claire des variables requises

---

### RISK-SYS-002 - Signatures ActivityPub Non Vérifiées

| Attribut | Valeur |
|----------|--------|
| STRIDE | S (Spoofing), T (Tampering) |
| Score | 80 (I:4 × P:5 × E:4) |
| Sévérité | CRITIQUE |

Les commentaires TODO indiquent que la vérification des signatures HTTP n'est pas implémentée, permettant l'acceptation d'activités forgées.

**Contrôles requis :**
- Implémenter vérification HTTP Signatures (draft-cavage)
- Valider signature avant traitement de toute activité
- Logger les tentatives avec signatures invalides

---

### RISK-SYS-003 - Clés Cryptographiques Placeholder

| Attribut | Valeur |
|----------|--------|
| STRIDE | S (Spoofing), I (Info Disclosure) |
| Score | 64 (I:4 × P:4 × E:4) |
| Sévérité | IMPORTANT |

Les clés publiques et privées ActivityPub sont des placeholders, rendant la fédération non fonctionnelle et potentiellement vulnérable.

**Contrôles requis :**
- Génération de paires RSA par utilisateur
- Stockage sécurisé des clés privées
- Rotation des clés si compromises

---

### RISK-SYS-004 - Absence d'Audit Logging

| Attribut | Valeur |
|----------|--------|
| STRIDE | R (Repudiation) |
| Score | 45 (I:3 × P:5 × E:3) |
| Sévérité | IMPORTANT |

Le système utilise tracing pour le logging applicatif mais n'a pas de piste d'audit structurée pour les actions sensibles (RGPD compliance).

**Contrôles requis :**
- Implémenter audit logging JSON structuré
- Logger toutes les actions sur données personnelles
- Intégrer un SIEM ou centraliser les logs

---

### RISK-SYS-005 - Exposition Données Privées via ActivityPub

| Attribut | Valeur |
|----------|--------|
| STRIDE | I (Information Disclosure) |
| Score | 36 (I:3 × P:4 × E:3) |
| Sévérité | MOYEN |

Les endpoints ActivityPub `/ap/recipes/:id` et `/ap/books/:id` ne vérifient pas la visibilité avant de retourner les objets.

**Contrôles requis :**
- Vérifier visibility avant exposition ActivityPub
- Ne pas exposer les recettes/books privés
- Retourner 404 pour objets privés

---

## Prochaine Étape

→ `/osk-analyze [feature]` pour analyse détaillée par feature

Ordre recommandé :
1. `/osk-analyze auth` (Critical)
2. `/osk-analyze federation` (Critical)
3. `/osk-analyze users` (High)
4. `/osk-analyze images` (High)
5. `/osk-analyze social` (High)
