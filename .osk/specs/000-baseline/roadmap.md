# Roadmap Sécurité

> Généré par `/osk-baseline` le 2025-12-26

## Objectif

Amener le projet de **36% (À risque)** à **75%+ (Acceptable)** de maturité sécurité.

## Vue d'ensemble

```
┌─────────────────────────────────────────────────────────────────────┐
│ PHASE 0        │ PHASE 1         │ PHASE 2         │ PHASE 3       │
│ Quick Wins     │ Fondations      │ Features Crit.  │ Couverture    │
│ 2-4h           │ 1-2j            │ 3-5j            │ 1-2sem        │
├─────────────────────────────────────────────────────────────────────┤
│ ▓▓░░░░░░░░░░░░ │ ░░░░░░░░░░░░░░░ │ ░░░░░░░░░░░░░░░ │ ░░░░░░░░░░░░░ │
│ 36% → 45%      │ 45% → 55%       │ 55% → 70%       │ 70% → 80%     │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Phase 0 : Quick Wins

**Objectif :** Corriger les vulnérabilités critiques immédiates

### Tâches

- [ ] **P0-001** : Crasher si `JWT_SECRET` non défini
  - Fichier : `src/api/middleware/auth.rs:87`
  - Remplacer `unwrap_or_else` par `expect`

- [ ] **P0-002** : Crasher si clés ActivityPub non configurées
  - Fichier : `src/jobs/federation.rs:127`
  - Ajouter validation au démarrage

- [ ] **P0-003** : Ajouter validation secrets au démarrage
  - Créer `src/lib/config.rs` avec validation
  - Vérifier longueur minimale JWT_SECRET (32 chars)

### Impact

- Score V (Secrets) : 20% → 50%
- Score global : 36% → 45%

---

## Phase 1 : Fondations

**Objectif :** Mettre en place l'infrastructure sécurité de base

### Tâches

- [ ] **P1-001** : Implémenter génération clés RSA ActivityPub
  - Générer paire RSA à la création utilisateur
  - Stocker en base de données (table `user_keys`)

- [ ] **P1-002** : Implémenter vérification HTTP Signatures
  - Parser header Signature entrant
  - Fetch clé publique de l'acteur distant
  - Valider signature avant traitement

- [ ] **P1-003** : Ajouter audit logging structuré
  - Créer trait `Auditable`
  - Logger JSON structuré pour actions sensibles
  - Inclure : actor, action, target, timestamp, ip

- [ ] **P1-004** : Configurer CORS explicitement
  - Ajouter tower-http CORS layer
  - Définir origines autorisées

### Impact

- Score I (Threat Modeling) : 30% → 40%
- Score III (Security Req) : 45% → 60%
- Score VI (Audit) : 25% → 50%
- Score global : 45% → 55%

---

## Phase 2 : Features Critiques

**Objectif :** Analyser en détail les features à haut risque

### Tâches

- [ ] **P2-001** : `/osk-analyze auth`
  - Analyse STRIDE détaillée auth JWT
  - Exigences sécurité
  - Tests de sécurité

- [ ] **P2-002** : `/osk-analyze federation`
  - Analyse STRIDE ActivityPub
  - Hardening inbox/outbox
  - Validation des activités

- [ ] **P2-003** : `/osk-analyze images`
  - Analyse upload de fichiers
  - Validation MIME types
  - Sanitization noms de fichiers

### Impact

- Score I (Threat Modeling) : 40% → 60%
- Score II (Risk Analysis) : 30% → 60%
- Score III (Security Req) : 60% → 75%
- Score global : 55% → 70%

---

## Phase 3 : Couverture Complète

**Objectif :** Atteindre une couverture sécurité mature

### Tâches

- [ ] **P3-001** : `/osk-analyze users`
  - Conformité RGPD
  - Droits des personnes

- [ ] **P3-002** : `/osk-analyze social`
  - Relations et activités
  - IDOR protection

- [ ] **P3-003** : `/osk-analyze recipes`
  - CRUD validation
  - Visibility enforcement

- [ ] **P3-004** : `/osk-analyze books`
  - Collections validation
  - Access control

- [ ] **P3-005** : Implémenter SAST/DAST
  - Intégrer cargo-clippy sécurité
  - Ajouter fuzzing tests

- [ ] **P3-006** : Documentation sécurité
  - SECURITY.md
  - Politique de divulgation

### Impact

- Score I : 60% → 80%
- Score II : 60% → 80%
- Score III : 75% → 85%
- Score IV : 40% → 70%
- Score global : 70% → 80%

---

## Métriques de Suivi

### KPI Sécurité

| Métrique | Baseline | Cible Phase 1 | Cible Final |
|----------|----------|---------------|-------------|
| Vulns Critiques | 3 | 0 | 0 |
| Vulns Importantes | 1 | 0 | 0 |
| Score Global | 36% | 55% | 80% |
| Features analysées | 0/8 | 0/8 | 8/8 |
| Couverture audit | 0% | 50% | 100% |

### Commandes de Suivi

```bash
# Dashboard de progression
/osk-dashboard

# Statut des vulnérabilités
/osk-risks

# Analyse d'une feature
/osk-analyze [feature]
```

---

## Prochaine Action

Exécuter **Phase 0** immédiatement :

```bash
# Fixer les vulnérabilités critiques
/osk-harden
```
