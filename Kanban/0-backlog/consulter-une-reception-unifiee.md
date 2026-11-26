---
id: ki7yh3h6s
title: Consulter une réception unifiée
depends_on: ks5rethb9, k1x8jlzjn, kgv7oozd2
---

## Contexte
Webmail Infomaniak à interface HEY (back Rust dans `server/`, front SolidJS dans `web/`, plusieurs boîtes par utilisateur). Cahier : `Kanban/.specs/piece-TlW9Y4/cahier-des-charges-webmail-infomaniak-hey.md`, carte R05, ligne « Comptes liés et réception unifiée ». Jalon Parité : relis le code actuel des vues avant de commencer.

## À faire
- `GET /api/v1/views/reception?unifiee=1` : fusionne les Réceptions des comptes de l'utilisateur, avec un curseur stable sur (date, compte, identifiant) et les mêmes sections « Nouveaux » et « Déjà vus ». Les décisions du tri restent celles de chaque boîte.
- Chaque résumé porte son compte d'origine.
- Front : choix « Toutes les boîtes » dans le sélecteur de compte ; chaque ligne affiche un repère de sa boîte, lisible sans la couleur.
- Les actions utilisent toujours le compte de l'élément : archiver va dans les Archives de la bonne boîte ; répondre utilise l'identité de la boîte qui a reçu le message.
- Une sélection mêlant deux boîtes produit une opération par boîte, avec un résultat par élément.
- Le flux d'événements de la vue unifiée agrège les révisions de chaque compte sans les mélanger.

## Critères d'acceptation
- [ ] Chaque ligne indique sa boîte.
- [ ] Répondre à un message de la boîte B part avec l'identité de B.
- [ ] Archiver un message de A et un de B place chacun dans les Archives de sa boîte (vérification IMAP).
- [ ] Un expéditeur accepté dans A et inconnu dans B reste en attente pour B.

## Vérification
```
make check
make test-integration
```

## Dépendances
« Archiver une sélection et garder la place de lecture », « Répondre, répondre à tous et transférer », « Rattacher une deuxième boîte ».