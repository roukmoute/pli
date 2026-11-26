---
id: kbissnj87
title: Paginer et restaurer la position
depends_on: kil5ghasf
---

## Contexte
Webmail Infomaniak à interface HEY (front SolidJS dans `web/`, module `web/src/sync/`). Cahier : `Kanban/.specs/piece-TlW9Y4/cahier-des-charges-webmail-infomaniak-hey.md`, carte L02, ligne « Trier, paginer ou remonter ». L'API pagine par curseur ; la liste n'affiche que la première page.

## À faire
- Charger les pages suivantes par curseur à l'approche du bas de la liste, avec un bouton de repli accessible au clavier.
- Conserver par vue, dans le store de `web/src/sync/` : tri, filtre, section, pages chargées, ancre de défilement et ligne au focus.
- Fusion des pages par identifiant stable : jamais de doublon, ordre départagé par l'identifiant à date identique.
- Si un message arrive pendant la consultation d'une page ancienne : la ligne sous le pointeur ne bouge pas et un accès « Nouveautés » apparaît en haut.
- Conserver l'ancre de défilement lors d'un ajout de lignes ou d'un changement de hauteur.
- Les vues de dossier (Envoyés, Archives, dossiers personnels) utilisent le même mécanisme.

## Critères d'acceptation
- [ ] Deux pages successives de messages à dates identiques forment un ensemble stable et complet.
- [ ] Retour depuis un autre dossier : le focus revient sur la bonne ligne, y compris en page 3.
- [ ] Une arrivée pendant la lecture d'une page ancienne ne décale pas la ligne survolée.
- [ ] La taille de page vient d'un réglage, indépendant de la taille des lots de synchronisation.

## Vérification
```
make check
```

## Dépendances
« Afficher la Réception et son menu ».