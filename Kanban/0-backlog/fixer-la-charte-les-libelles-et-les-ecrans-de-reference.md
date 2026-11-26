---
id: k0ym6scsi
title: Fixer la charte, les libellés et les écrans de référence
depends_on: kdbrmneuv, kmu2pmyh6
---

## Contexte
Webmail Infomaniak à interface HEY (front SolidJS/TypeScript strict + CSS Modules dans `web/`). Cahier : `Kanban/.specs/piece-TlW9Y4/cahier-des-charges-webmail-infomaniak-hey.md`, carte F01 et section « Écrans et navigation inspirés de HEY ». Les écrans suivants ont besoin d'une référence commune : couleurs, libellés français, icônes, et une planche qui relie chaque écran aux captures officielles de HEY.

## À faire
- `docs/reference/planche.md`, datée : pour Imbox, menu, Screener, fil de lecture et Fichiers, le lien vers la capture officielle citée dans le cahier (liens uniquement : ne copie aucune image ni icône de HEY dans le dépôt), la composition attendue, les libellés français et les états à vérifier aux formats 390 × 844, 768 × 1024 et 1440 × 900.
- `web/src/styles/tokens.css` : variables CSS de la charte du cahier (fond clair, surface de lecture blanche aux angles arrondis, titres foncés, commandes indigo, signal turquoise pour le tri), espacements, typographie, largeur de lecture, densité, anneau de focus. Contraste AA vérifié par calcul.
- `web/src/i18n/` : dictionnaire `fr` avec tous les libellés du cahier (Réception, Nouveaux, Déjà vus, Flux, Justificatifs, Répondre plus tard, De côté, Fichiers, Écrire, Accepter, Refuser, Déplacer vers Indésirables…) et une fonction `t()` typée, prête à accueillir d'autres langues.
- `web/src/icons/` : icônes SVG propres au projet (menu, recherche, écrire, étoile, pièce jointe, répondre, transférer, archive, corbeille, dossier, plus), en composants Solid avec nom accessible obligatoire.
- Composants de base `Bouton`, `BoutonIcone`, `Avatar` (image ou initiales) avec leurs tests.

## Critères d'acceptation
- [ ] La planche couvre les cinq écrans, les trois formats et cite ses sources par lien.
- [ ] Aucun asset de HEY n'est dans le dépôt.
- [ ] Un test échoue si une clé de libellé utilisée n'existe pas dans `fr`.
- [ ] Chaque icône exige un nom accessible ; l'état nouveau, la sélection et le focus ont un signe indépendant de la couleur dans les jetons.

## Vérification
```
make check
```

## À vérifier par l'humain
- Les couleurs et la typographie conviennent ; les droits sur les ressources visuelles sont acceptables.

## Dépendances
« Poser le socle Rust/Axum et SolidJS/Vite ».