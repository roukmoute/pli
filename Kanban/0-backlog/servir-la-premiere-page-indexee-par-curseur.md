---
id: kpji7rvfl
title: Servir la première page indexée par curseur
depends_on: kjdx6zusm
---

## Contexte
Webmail Infomaniak à interface HEY (back Rust/Axum/rusqlite dans `server/`). Cahier : `Kanban/.specs/piece-TlW9Y4/cahier-des-charges-webmail-infomaniak-hey.md`, carte F06, paragraphe « Contrat entre front et back », ligne « Résumés et recherche ». L'API sert les listes depuis SQLite, sans appel IMAP, et applique la décision du Screener avant de préparer listes et compteurs.

## À faire
- `GET /api/v1/views/{vue}` pour `reception`, `flux`, `justificatifs`, et `GET /api/v1/folders/{id}/messages` pour un dossier. Paramètres `taille` (défaut 20) et `curseur`.
- Réponse : compte, vue ou dossier, résumés de fils par section (`nouveaux` : fils avec au moins un message non lu admis ; `deja_vus` : fils lus), `curseur_suivant`, `revision`, `couverture` de synchronisation (import en cours, proportion indexée), compteurs par destination et nombre d'expéditeurs en attente.
- Résumé de fil : identifiant, objet, expéditeur (nom, adresse), extrait, date, nombre de messages, indicateurs (non lu, étoile, pièce, répondu, transféré).
- Pagination par curseur sur (date de réception, identifiant stable) avec l'index SQL correspondant ; jamais de `OFFSET`.
- Filtrage côté serveur : les messages d'expéditeurs en attente ou refusés n'entrent dans aucune liste ni compteur ordinaires.
- Migration `account_revisions` : compteur de révision par compte, incrémenté dans la même transaction que toute modification de la projection. Adapter l'import pour l'incrémenter.

## Critères d'acceptation
- [ ] Les premiers résumés admis sont servis avant la fin de l'import, avec une couverture partielle annoncée.
- [ ] Deux pages successives de messages à dates identiques ne se recouvrent pas et n'oublient rien.
- [ ] Les compteurs par destination sont justes après une reprise d'import.
- [ ] Aucune requête de cette API n'ouvre de connexion IMAP.
- [ ] Test de performance (non lancé par défaut) : page de 50 résumés au milieu de 100 000 messages fictifs en moins de 5 ms au p95.

## Vérification
```
make check
make test-integration
```

## Dépendances
« Importer les en-têtes et trier les nouveaux expéditeurs ».