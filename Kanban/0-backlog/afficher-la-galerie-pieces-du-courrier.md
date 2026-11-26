---
id: kwwlmwlbk
title: Afficher la galerie « Pièces du courrier »
depends_on: k9mxiyet6
---

## Contexte
Webmail Infomaniak à interface HEY (back Rust dans `server/`, front SolidJS dans `web/`, service des pièces via `GET /api/v1/messages/{id}/parts/{partie}`). Cahier : `Kanban/.specs/piece-TlW9Y4/cahier-des-charges-webmail-infomaniak-hey.md`, carte X02 (première partie), ligne « Fichiers et pièces jointes ». La tuile « Fichiers » du menu ne mène nulle part.

## À faire
- Migration `attachments_index` : pièces de chaque message (compte, message, partie, nom, type, taille, date, expéditeur), alimentée à l'import depuis la structure MIME et retirée avec le message.
- `GET /api/v1/files/attachments` : filtres par type et expéditeur, tri par nom, taille ou date, pagination par curseur. Les pièces des messages d'expéditeurs en attente ou refusés sont exclues.
- Page Fichiers, chargée à sa première utilisation, avec deux onglets : « Pièces du courrier » (cette carte) et « Mes fichiers » (onglet vide annoncé).
- Galerie : nom, type, taille, date, expéditeur, lien vers le message source dans son fil, aperçu et téléchargement.
- Cartes chargées par lots ; miniatures chargées à l'approche de la zone visible ; une miniature lente garde sa place sans retarder le texte.

## Critères d'acceptation
- [ ] Le PDF du jeu `base` apparaît dans la galerie et son lien ouvre le message source.
- [ ] Les filtres par type et par expéditeur se combinent.
- [ ] Une pièce d'un message en attente au Screener n'apparaît pas.
- [ ] Le code de la page Fichiers n'est pas dans le chargement initial (vérification du découpage du build).
- [ ] Aucune pièce d'un autre compte n'est listée.

## Vérification
```
make check
make test-integration
```

## À vérifier par l'humain
- Rendu de la galerie comparé à la planche de référence.

## Dépendances
« Télécharger et prévisualiser une pièce reçue ».