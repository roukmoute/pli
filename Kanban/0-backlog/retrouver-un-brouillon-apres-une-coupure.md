---
id: kd9c3y4qa
title: Retrouver un brouillon après une coupure
depends_on: k9g91mi7z, kklmrw0if
---

## Contexte
Webmail Infomaniak à interface HEY (front SolidJS dans `web/`, cache IndexedDB dans `web/src/sync/cache.ts`, brouillons serveur via `PUT /api/v1/drafts/{id}`). Cahier : `Kanban/.specs/piece-TlW9Y4/cahier-des-charges-webmail-infomaniak-hey.md`, carte C02, paragraphe « Brouillons ». La perte de brouillon est une anomalie bloquante de livraison.

## À faire
- Enregistrer le brouillon dans IndexedDB à chaque modification (avec un court délai de regroupement), via `web/src/sync/cache.ts`, sur appareil de confiance. État « enregistré sur cet appareil ».
- Synchronisation automatique vers le serveur en arrière-plan, avec les trois états du cahier : « enregistré sur cet appareil », « synchronisation en cours », « enregistré dans Brouillons ».
- Au rechargement ou après une coupure réseau : retrouver le texte, puis synchroniser la dernière version.
- Conflit (deux onglets modifient le même brouillon, version attendue refusée) : conserver les deux versions, la seconde devenant un brouillon distinct clairement identifié comme version en conflit.
- Les brouillons locaux ne sont jamais évincés avant d'être synchronisés.
- Composeur minimisable : réduit en bas de l'écran, il laisse consulter un autre message, puis se rouvre dans le même état.

## Critères d'acceptation
- [ ] Du texte saisi puis un rechargement immédiat : le texte est là.
- [ ] Saisie hors ligne, puis retour du réseau : la dernière version arrive dans Brouillons.
- [ ] Deux onglets modifient le même brouillon : les deux versions existent et sont identifiables.
- [ ] Minimiser, ouvrir un autre fil, reprendre : curseur et contenu conservés.

## Vérification
```
make check
make test-integration
```

## Dépendances
« Mettre vues et fils récents en cache IndexedDB », « Composer un premier brouillon ».