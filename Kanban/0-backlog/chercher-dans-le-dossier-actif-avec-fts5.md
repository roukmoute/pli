---
id: kqhbe3h9k
title: Chercher dans le dossier actif avec FTS5
depends_on: kbissnj87, kx7bmitqo
---

## Contexte
Webmail Infomaniak à interface HEY (back Rust/rusqlite dans `server/`, front SolidJS dans `web/`). Cahier : `Kanban/.specs/piece-TlW9Y4/cahier-des-charges-webmail-infomaniak-hey.md`, carte R01, paragraphe « Recherche », ligne « Résumés et recherche ». La loupe de l'en-tête est un bouton provisoire.

## À faire
- Migration : table FTS5 (objet, expéditeur, destinataires, texte du corps quand il est disponible), segmentée par compte. Alimentation à l'import ; suppression des entrées quand un message est retiré ; un corps arrivé plus tard enrichit l'entrée existante sans créer de seconde ligne.
- `GET /api/v1/search` : requête, périmètre (vue courante ou dossier), pagination par curseur, résultats regroupés par fil avec le message précis trouvé, couverture de l'index.
- Les messages d'expéditeurs en attente ou refusés sont exclus, sauf filtre explicite `inclure_tri`.
- Front : la loupe ouvre la recherche ; une première liste courte aide à reprendre un fil ; « Tous les résultats » ouvre la page complète. Effacer la recherche restitue la vue précédente avec sa position.
- Chaque requête porte un numéro : une réponse plus ancienne arrivée en retard est ignorée.
- Index incomplet : afficher sa couverture. « Aucun résultat » n'apparaît que pour une recherche terminée, avec le périmètre affiché.

## Critères d'acceptation
- [ ] Une recherche par objet, expéditeur ou texte renvoie les fils du bon compte, paginés.
- [ ] Un message refusé n'apparaît que si le périmètre du tri est demandé.
- [ ] Une réponse tardive d'une requête précédente n'écrase pas les résultats courants.
- [ ] Ouvrir un résultat mène au message précis dans son fil.
- [ ] Test de performance (non lancé par défaut) : première page en moins de 300 ms sur 100 000 messages fictifs, index chaud.

## Vérification
```
make check
make test-integration
```

## Dépendances
« Paginer et restaurer la position ».