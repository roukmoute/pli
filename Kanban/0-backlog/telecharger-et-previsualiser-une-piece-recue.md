---
id: k9mxiyet6
title: Télécharger et prévisualiser une pièce reçue
depends_on: kx7bmitqo
---

## Contexte
Webmail Infomaniak à interface HEY (back Rust dans `server/`, front SolidJS dans `web/`, fil de lecture existant). Cahier : `Kanban/.specs/piece-TlW9Y4/cahier-des-charges-webmail-infomaniak-hey.md`, carte C06, ligne « Lire un contenu complexe », section « Frontières de confiance ». Le fil liste les pièces mais ne permet pas de les ouvrir.

## À faire
- `GET /api/v1/messages/{id}/parts/{partie}` : lit la partie MIME en flux depuis IMAP (`BODY.PEEK`), après vérification du compte. Type servi depuis une liste sûre, sinon `application/octet-stream` ; `Content-Disposition: attachment` par défaut ; `X-Content-Type-Options: nosniff`.
- Aperçu des types compatibles (images, PDF, texte) dans un contexte isolé : `iframe` avec `sandbox` et politique de sécurité de contenu stricte. Jamais d'exécution de HTML ou SVG reçu.
- Front : chaque pièce du fil montre nom, type, taille, Télécharger et Aperçu. Téléchargement avec progression et annulation.
- Le lecteur de pièces est un module chargé à sa première utilisation.
- La récupération d'une pièce ne retarde pas l'affichage du texte du message : elle ne commence qu'à la demande.

## Critères d'acceptation
- [ ] Le PDF du jeu `base` se télécharge à l'identique (empreinte comparée).
- [ ] Une pièce `.html` ou `.svg` n'est jamais rendue dans l'origine de l'application.
- [ ] Une pièce volumineuse est servie en flux, sans chargement complet en mémoire.
- [ ] La pièce d'un autre compte renvoie 404.
- [ ] Ouvrir le message ne déclenche aucun téléchargement de pièce.

## Vérification
```
make check
make test-integration
```

## À vérifier par l'humain
- L'aperçu d'une image et d'un PDF dans le navigateur.

## Dépendances
« Ouvrir un message assaini et retrouver la liste ».