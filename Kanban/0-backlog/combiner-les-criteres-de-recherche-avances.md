---
id: k79wmd738
title: Combiner les critères de recherche avancés
depends_on: kqhbe3h9k
---

## Contexte
Webmail Infomaniak à interface HEY (back Rust dans `server/`, front SolidJS dans `web/`, recherche FTS5 via `GET /api/v1/search`). Cahier : `Kanban/.specs/piece-TlW9Y4/cahier-des-charges-webmail-infomaniak-hey.md`, carte R02, paragraphe « Recherche ».

## À faire
- Étendre `GET /api/v1/search` : expéditeur, destinataire, objet, texte, bornes de dates, présence de pièce jointe et nom de pièce (indexer les noms de pièces).
- Périmètres : vue courante, dossier, dossier avec ses sous-dossiers, ensemble des dossiers du compte.
- Front : panneau avancé dans la page de résultats, avec les filtres actifs visibles et retirables un par un.
- Index partiel : signaler la couverture et proposer « poursuivre en ligne », qui lance une recherche `SEARCH` IMAP sur la partie non indexée et fusionne les résultats sans doublon.
- « Aucun résultat » rappelle le périmètre et les filtres appliqués, et propose de l'élargir.

## Critères d'acceptation
- [ ] Dates, destinataire, pièce jointe et dossier étendu se combinent correctement.
- [ ] Le périmètre « dossier avec sous-dossiers » inclut bien les sous-dossiers.
- [ ] Avec un index partiel, la couverture est annoncée et la recherche en ligne complète les résultats.
- [ ] Zéro résultat explique le périmètre.
- [ ] Aucun résultat ne provient d'un autre compte.

## Vérification
```
make check
make test-integration
```

## Dépendances
« Chercher dans le dossier actif avec FTS5 ».