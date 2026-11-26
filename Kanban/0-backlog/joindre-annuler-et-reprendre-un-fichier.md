---
id: ktzs898x5
title: Joindre, annuler et reprendre un fichier
depends_on: kklmrw0if
---

## Contexte
Webmail Infomaniak à interface HEY (back Rust/Axum dans `server/`, front SolidJS dans `web/`, composeur et brouillons existants). Cahier : `Kanban/.specs/piece-TlW9Y4/cahier-des-charges-webmail-infomaniak-hey.md`, carte C05, paragraphe « Brouillons », ligne « Fichiers et liens de partage ».

## À faire
- Stockage des fichiers applicatifs dans `WEBMAIL_DATA_DIR/files`, rattachés à un compte. Migration `draft_attachments` (brouillon, nom, type, taille, état, emplacement).
- `POST /api/v1/drafts/{id}/attachments` : envoi en flux, par morceaux reprenables, avec limite de taille configurable (`WEBMAIL_MAX_ATTACHMENT_MB`). `DELETE` pour retirer.
- Front : bouton et glisser-déposer dans le composeur ; chaque pièce affiche nom, taille, progression, erreur éventuelle, et les commandes Annuler, Reprendre, Retirer.
- Un échec de transfert ne touche ni au texte ni aux autres pièces.
- Après une coupure ou un rechargement, distinguer les pièces déjà sur le serveur de celles qui doivent être jointes à nouveau.
- Un fichier dépassant la limite est refusé avec la taille autorisée.

## Critères d'acceptation
- [ ] Le transfert affiche sa progression et peut être annulé.
- [ ] Après un échec simulé, le texte reste et le fichier peut être repris ou retiré.
- [ ] Après rechargement, une pièce incomplète est signalée « à joindre à nouveau ».
- [ ] Un fichier trop gros est refusé sans perdre le brouillon.
- [ ] La pièce d'un compte n'est pas accessible depuis un autre.

## Vérification
```
make check
```

## Dépendances
« Composer un premier brouillon ».