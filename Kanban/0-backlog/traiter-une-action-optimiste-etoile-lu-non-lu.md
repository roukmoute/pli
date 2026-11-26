---
id: kirtkkako
title: "Traiter une action optimiste : étoile, lu, non lu"
depends_on: k7gwk72hq
---

## Contexte
Webmail Infomaniak à interface HEY (back Rust dans `server/`, front SolidJS dans `web/`, module `web/src/sync/`). Cahier : `Kanban/.specs/piece-TlW9Y4/cahier-des-charges-webmail-infomaniak-hey.md`, carte L03, ligne « Changements et actions locales », paragraphe « Contrat entre front et back ». Cette carte crée le mécanisme d'opérations idempotentes que le classement, l'envoi et les piles réutiliseront.

## À faire
- Migration `operations` : identifiant fourni par le client (UUID), compte, type, cibles, état attendu, état (`en_attente`, `confirmee`, `refusee`), résultat par élément.
- `POST /api/v1/operations` : types `etoile`, `sans_etoile`, `lu`, `non_lu`. La projection SQLite, l'enregistrement de l'opération et la nouvelle révision du compte sont écrits dans une seule transaction ; la réponse est immédiate. Rejouer le même identifiant rend le résultat connu sans rien réappliquer. `GET /api/v1/operations/{id}`.
- Le synchroniseur applique l'opération en IMAP (`STORE`), puis la confirme ou la refuse ; un refus rétablit la projection pour les seuls éléments concernés.
- Deux intentions opposées en attente (lu puis non lu) convergent vers la dernière.
- Front : `web/src/sync/operations.ts`, mécanisme générique (appliquer localement, envoyer, confirmer ou annuler). Une action en attente a un indicateur distinct de l'action confirmée, lisible sans la couleur.
- Brancher l'étoile et lu/non lu dessus ; remplacer l'appel direct `POST /api/v1/messages/flags` de la sélection multiple. Étoiler ne redessine que l'étoile.

## Critères d'acceptation
- [ ] Étoile, lu et non lu réagissent au clic sans attendre le réseau.
- [ ] La même opération envoyée deux fois donne le même résultat et un seul `STORE`.
- [ ] Un refus IMAP simulé rétablit le bon état et le signale.
- [ ] Lu puis non lu rapprochés aboutissent à non lu, côté webmail et côté IMAP.
- [ ] Une opération visant un message d'un autre compte est refusée.

## Vérification
```
make check
make test-integration
```

## Dépendances
« Distinguer l'ouverture du fil et la sélection multiple ».