---
id: ki4xl91qt
title: Initialiser le dépôt git
---

## Contexte
Le projet est un webmail Infomaniak à interface inspirée de HEY (back Rust, front SolidJS/TypeScript). Le dossier ne contient pour l'instant que `Kanban/` (tableau et specs) et n'est pas un dépôt git. Les agents commitent le travail de chaque carte : il faut d'abord le dépôt.

## À faire
- `git init`, branche par défaut `main`.
- Créer `.gitignore` : `target/`, `node_modules/`, `dist/`, `web/dist/`, `coverage/`, `playwright-report/`, `test-results/`, `data/`, `*.db`, `*.db-wal`, `*.db-shm`, `.env`, `.env.*` (sauf `.env.example`), `*.pem`, `*.key`, `Kanban/.runs/`, `Kanban/.leases/`, `Kanban/.downloads/`, `Kanban/.lead/`, fichiers d'éditeur et d'OS.
- Créer un `README.md` minimal (nom du projet, une phrase d'objectif, renvoi au cahier `Kanban/.specs/piece-TlW9Y4/cahier-des-charges-webmail-infomaniak-hey.md`).
- Premier commit contenant `.gitignore`, `README.md` et `Kanban/` (cartes et specs).

## Critères d'acceptation
- [x] Le dépôt existe sur la branche `main` avec un premier commit.
- [x] `.env.recette`, `Kanban/.runs/`, `Kanban/.leases/`, `Kanban/.downloads/`, `Kanban/.lead/`, `target/` et `node_modules/` sont ignorés.
- [x] Aucun secret n'est commité.

## Vérification
```
git branch --show-current
git log --oneline | head -1
git check-ignore .env.recette Kanban/.runs/x Kanban/.leases/x Kanban/.downloads/x Kanban/.lead/x target/x node_modules/x
git status --short
```
`git check-ignore` doit lister les sept chemins ; `git status --short` doit être vide.

## Dépendances
Aucune.
