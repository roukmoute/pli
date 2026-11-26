---
id: k950kpezy
title: Comparer le produit aux références HEY et Infomaniak
depends_on: kut392580, k79wmd738, kwg66u0bh, krsvz68mw, kp6c7794x, kwwlmwlbk
---

## Contexte
Webmail Infomaniak à interface HEY (planche dans `docs/reference/planche.md`, Playwright dans `e2e/`, mesures dans `docs/mesures/`, banc dans `bench/`). Cahier : `Kanban/.specs/piece-TlW9Y4/cahier-des-charges-webmail-infomaniak-hey.md`, carte A07, paragraphes « Recette visuelle », « Protocole reproductible » et « Preuves et comparaison ». L'agent n'a pas d'écran : il prépare le dossier de comparaison, l'humain juge le visuel.

## À faire
- Script Playwright `make captures` : sur la boîte fictive du banc, capturer aux trois formats (390 × 844, 768 × 1024, 1440 × 900) la Réception vide puis remplie, le menu ouvert, le Screener, deux lignes sélectionnées, un fil déplié, la rédaction avec pièce, la galerie Fichiers, et l'état après « archiver puis revenir ». Images rangées dans `docs/recette/captures/AAAA-MM-JJ/`.
- Contrôles automatiques sur ces écrans : ordre des actions, hiérarchie des titres, taille des cibles tactiles, contraste, focus visible, ancre de défilement.
- `docs/recette/comparaison.md` : pour chaque écran, la capture du produit à côté du lien vers la capture officielle de HEY, les points à comparer, et une colonne « écart accepté / à corriger » à remplir, chaque écart étant rattaché à un parcours.
- Reprendre du dernier rapport de `docs/mesures/` les latences des parcours comparables et préparer le protocole de mesure sur Infomaniak Mail (mêmes messages, navigateur, fenêtre, profil réseau), avec un tableau à compléter.
- Rappel dans le document : une comparaison « plus rapide » désigne un parcours, un concurrent, une configuration et une date.

## Critères d'acceptation
- [ ] `make captures` produit toutes les captures sans écran.
- [ ] Le document couvre chaque écran de la recette visuelle du cahier.
- [ ] Les contrôles automatiques passent ou leurs écarts sont listés.
- [ ] Aucune image de HEY n'est copiée dans le dépôt.

## Vérification
```
make captures
make check
```

## À vérifier par l'humain
- Comparer les captures aux vues officielles de HEY et remplir la colonne des écarts.
- Mesurer les parcours comparables sur Infomaniak Mail avec la boîte de recette et compléter le tableau.

## Dépendances
« Lire le Flux et retrouver un justificatif », « Combiner les critères de recherche avancés », « Travailler au clavier et au lecteur d'écran », « Mesurer les budgets de latence », « Administrer les comptes et diagnostiquer la synchronisation ».