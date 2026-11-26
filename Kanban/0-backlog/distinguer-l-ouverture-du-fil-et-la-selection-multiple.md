---
id: k7gwk72hq
title: "Distinguer l'ouverture du fil et la sélection multiple"
depends_on: kx7bmitqo
---

## Contexte
Webmail Infomaniak à interface HEY (front SolidJS dans `web/`, module `web/src/sync/`, back Rust dans `server/`). Cahier : `Kanban/.specs/piece-TlW9Y4/cahier-des-charges-webmail-infomaniak-hey.md`, carte L01, ligne « Ouvrir ou sélectionner ». La liste ouvre déjà un fil ; il faut pouvoir sélectionner plusieurs lignes sans confondre les deux gestes.

## À faire
- L'avatar de chaque ligne sert de sélecteur (rôle de case à cocher, nom accessible, utilisable au pointeur et au clavier). Un clic sur l'objet ou Entrée ouvre le fil. L'étoile garde sa propre cible.
- État de sélection par vue dans le store de `web/src/sync/`, indépendant du dernier fil ouvert.
- Sélection globale de la page avec état intermédiaire, et annonce du nombre d'éléments sélectionnés dans une zone `aria-live`.
- Barre d'actions de sélection, avec « Marquer comme lu » et « Marquer comme non lu ».
- Back : `POST /api/v1/messages/flags` (identifiants ciblés, indicateur, valeur) qui applique le changement en IMAP et retourne un résultat par élément.
- La sélection a un signe visible indépendant de la couleur.

## Critères d'acceptation
- [ ] Ouvrir A, revenir, sélectionner B et C par leur avatar, marquer lus : seuls B et C changent ; A garde son état et son ancre.
- [ ] La sélection partielle de page est annoncée avec la taille du lot.
- [ ] Cliquer l'étoile ou l'avatar n'ouvre pas le fil.
- [ ] Un identifiant d'un autre compte est refusé élément par élément.

## Vérification
```
make check
make test-integration
```

## À vérifier par l'humain
- Deux lignes sélectionnées comparées à la planche de référence.

## Dépendances
« Ouvrir un message assaini et retrouver la liste ».