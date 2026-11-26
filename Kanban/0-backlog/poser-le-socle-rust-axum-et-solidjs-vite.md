---
id: kdbrmneuv
title: Poser le socle Rust/Axum et SolidJS/Vite
depends_on: ki4xl91qt, kufl8r92q
---

## Contexte
Webmail Infomaniak à interface HEY. Cahier : `Kanban/.specs/piece-TlW9Y4/cahier-des-charges-webmail-infomaniak-hey.md` (sections « Architecture et cohérence des données » et « Caches et réactivité »). Le dépôt est vide : cette carte pose la structure et l'outillage que toutes les autres cartes utiliseront. Vérifie sur internet les versions ci-dessous avant d'installer et écris celles retenues dans le `README.md`.

## À faire
- **Back `server/`** : crate Rust (stable 1.100 fixée par `rust-toolchain.toml`), Axum 0.8.9, Tokio 1, `rusqlite` 0.40.x avec la feature `bundled`, `tracing`. Un seul binaire `webmail-server` (API + futur synchroniseur). Configuration par variables d'environnement : `WEBMAIL_LISTEN` (défaut `127.0.0.1:8080`), `WEBMAIL_DATA_DIR` (défaut `./data`). Fournir `.env.example`.
- **SQLite** : base dans `WEBMAIL_DATA_DIR`, mode WAL, `foreign_keys=ON`, une connexion d'écriture unique et un pool de lecture, migrations numérotées dans `server/migrations/` appliquées au démarrage.
- **API** : préfixe `/api/v1`, erreurs JSON `{code, message}`, `GET /api/v1/health`. Réponses d'API en `Cache-Control: no-store`. Le binaire sert aussi `web/dist` (fichiers à empreinte en cache long, `index.html` sans cache).
- **Front `web/`** : Node 24, Vite 8.1, SolidJS 2.0 avec `@solidjs/vite-plugin` (vérifier avec `npm view` les dist-tags et la compatibilité Vite 8 ; en cas de blocage, repli sur `solid-js` 1.9.x + `vite-plugin-solid` et le noter dans le README), TypeScript `strict: true`, CSS Modules, Biome 2.5.x. Proxy de dev `/api` vers le back. Une page d'accueil provisoire qui affiche le résultat de `/api/v1/health`.
- **Module de synchronisation `web/src/sync/`** : seule porte d'entrée du front vers le réseau et le stockage. Poser `api.ts` (client HTTP typé) et un `index.ts` qui exporte des actions métier. Règle écrite dans le README : aucun composant n'appelle `fetch`, IndexedDB ou EventSource directement.
- **`Makefile`** à la racine : `make dev`, `make lint` (`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `biome check`, `tsc --noEmit`), `make build` (front puis back en release), `make check` (lint + build).

## Critères d'acceptation
- [ ] `make check` passe sur un clone propre.
- [ ] Le binaire démarre, crée la base en WAL, applique les migrations et répond sur `/api/v1/health`.
- [ ] Le binaire sert le front construit ; les fichiers JS/CSS portent une empreinte.
- [ ] Le README décrit la structure, les commandes, les versions retenues et la règle du module de synchronisation.

## Vérification
```
make check
./server/target/release/webmail-server & sleep 2
curl -s localhost:8080/api/v1/health
curl -sI localhost:8080/ | head -5
kill %1
```

## Dépendances
« Initialiser le dépôt git ».