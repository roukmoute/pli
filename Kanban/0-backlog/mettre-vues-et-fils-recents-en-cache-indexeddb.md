---
id: k9g91mi7z
title: Mettre vues et fils récents en cache IndexedDB
depends_on: kx7bmitqo
---

## Contexte
Webmail Infomaniak à interface HEY (front SolidJS/TypeScript dans `web/`, module `web/src/sync/`). Cahier : `Kanban/.specs/piece-TlW9Y4/cahier-des-charges-webmail-infomaniak-hey.md`, carte S01, tableau « Caches et réactivité » (lignes « Mémoire de l'onglet » et « IndexedDB sur appareil de confiance »). Le store vit en mémoire ; à la seconde ouverture, tout est rechargé depuis le réseau.

## À faire
- `web/src/sync/cache.ts` : couche IndexedDB unique du projet (les brouillons et opérations en attente l'utiliseront aussi). Prévoir un schéma versionné avec migrations. Choisir entre l'API native et une petite bibliothèque après vérification de sa version.
- Contenu : listes récentes par vue, derniers corps assainis, préférences de présentation, dernière révision appliquée. Cloisonnement par utilisateur, compte et vue.
- Case « Appareil de confiance » à la connexion : sans elle, aucun cache persistant.
- Ouverture : dessiner l'interface et la liste depuis le cache, puis revalider en arrière-plan, sans appel bloquant. Une lecture servie par le cache indique sa fraîcheur.
- Invalidation : un changement de compte écarte les données de l'ancien ; un changement de révision du tri retire les lignes sorties de la vue avant de réafficher ; la déconnexion vide le cache.
- Tests avec `fake-indexeddb`.

## Critères d'acceptation
- [ ] Une seconde ouverture restitue section, ancre et contenu d'un fil déjà lu avant toute réponse réseau.
- [ ] Après changement de compte, aucune donnée de l'ancien compte n'est lisible.
- [ ] Un expéditeur refusé depuis un autre appareil disparaît de la vue à la reconnexion, sans apparition transitoire.
- [ ] Sans « appareil de confiance », IndexedDB reste vide.
- [ ] Aucun composant n'accède à IndexedDB hors de `web/src/sync/`.

## Vérification
```
make check
```

## Dépendances
« Ouvrir un message assaini et retrouver la liste ».