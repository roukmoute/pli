---
id: kvm7bk39k
title: Parcourir une conversation au clavier
depends_on: kx7bmitqo, kbissnj87
---

## Contexte
Webmail Infomaniak à interface HEY (front SolidJS dans `web/`, fil de lecture et liste paginée existants). Cahier : `Kanban/.specs/piece-TlW9Y4/cahier-des-charges-webmail-infomaniak-hey.md`, carte L07, lignes « Lire et revenir » et « Ouvrir un brouillon ou déplier un fil ». Jalon Parité : relis le code actuel du fil avant de commencer, il a pu évoluer.

## À faire
- Dans la liste, Entrée ouvre le fil de la ligne au focus.
- Dans le fil, chaque en-tête de message est focalisable ; Entrée sur un en-tête déplie ou replie ce message, sans déplacer la position de lecture. Les flèches passent d'un message à l'autre.
- Une commande d'ouverture séparée conserve le lien vers le message actif.
- Une nouvelle réponse admise fait réapparaître le fil en haut de « Nouveaux », y compris pendant qu'on consulte la liste.
- Retour : restaure la ligne d'origine ; si le fil a changé de section, place le focus sur son voisin à la position de départ.
- Annonce pour lecteur d'écran de l'état déplié ou replié (`aria-expanded`).

## Critères d'acceptation
- [ ] Entrée ouvre le fil, puis déplie le message dont l'en-tête a le focus.
- [ ] Déplier un message antérieur ne fait pas sauter le défilement.
- [ ] Une réponse nouvelle dans un fil lu le ramène dans « Nouveaux ».
- [ ] Après lecture du dernier non-lu, Retour place le focus sur le voisin de l'ancienne position.

## Vérification
```
make check
```

## Dépendances
« Ouvrir un message assaini et retrouver la liste », « Paginer et restaurer la position ».