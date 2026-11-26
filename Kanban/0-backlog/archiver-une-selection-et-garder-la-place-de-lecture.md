---
id: ks5rethb9
title: Archiver une sélection et garder la place de lecture
depends_on: kirtkkako
---

## Contexte
Webmail Infomaniak à interface HEY (back Rust dans `server/`, front SolidJS dans `web/`, opérations idempotentes via `POST /api/v1/operations` et `web/src/sync/operations.ts`). Cahier : `Kanban/.specs/piece-TlW9Y4/cahier-des-charges-webmail-infomaniak-hey.md`, section « Exemple de carte prête à développer » (L04) et ligne « Archiver, signaler ou supprimer ». Cette carte suit le tableau de cet exemple.

## À faire
- Nouveau type d'opération `archiver` (portée message, fil ou sélection). Le synchroniseur déplace vers le dossier de rôle Archives par `MOVE`, ou par `COPY` + `\Deleted` + `EXPUNGE` ciblé si MOVE est absent. Le nouvel UID est rattaché au message quand le serveur le communique.
- Si le rôle Archives est inconnu, demander la destination à l'utilisateur et la mémoriser pour le compte.
- Résultat par élément : un refus partiel rétablit uniquement les éléments refusés, qui sont seuls proposés à la reprise.
- Front : commande Archiver dans la barre de sélection et dans le fil. Les lignes quittent la liste immédiatement, les compteurs de non-lus s'ajustent, un état transitoire est visible.
- Après archivage du message ouvert : ouvrir le suivant si la lecture en série est active, sinon revenir à la liste sur la voisine de la ligne retirée ; si la liste est vide, le focus va au titre.
- Un libellé indique la portée (message, fil ou sélection) avant l'action.

## Critères d'acceptation
- [ ] A consulté, B et C sélectionnés, Archiver : B et C disparaissent tout de suite, A garde sa section et son état de lecture.
- [ ] Le serveur confirme B et refuse C : B est dans Archives, C revient en Réception, seul C est proposé à la reprise.
- [ ] La même opération rejouée rend l'état connu, sans second déplacement.
- [ ] Lire un message ne l'archive jamais : seule la commande déplace.
- [ ] Le dossier Archives contient bien les messages (lecture IMAP dans le test).

## Vérification
```
make check
make test-integration
```

## À vérifier par l'humain
- La ligne disparaît sans délai perceptible.

## Dépendances
« Traiter une action optimiste : étoile, lu, non lu ».