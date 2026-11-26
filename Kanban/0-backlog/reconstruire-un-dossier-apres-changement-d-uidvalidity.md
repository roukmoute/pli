---
id: khf35f65q
title: "Reconstruire un dossier après changement d'UIDVALIDITY"
depends_on: kirtkkako, kmqbjnt1y
---

## Contexte
Webmail Infomaniak à interface HEY (back Rust dans `server/`, synchroniseur dans `server/src/sync/`, banc dans `bench/`). Cahier : `Kanban/.specs/piece-TlW9Y4/cahier-des-charges-webmail-infomaniak-hey.md`, carte S04, paragraphe « Synchroniser vite et juste », lignes « Message et indicateurs » et « Tri des expéditeurs ». Un changement de génération IMAP rend tous les anciens UID d'un dossier invalides.

## À faire
- Resynchronisation à la reconnexion : utiliser QRESYNC ou CONDSTORE si annoncés ; sinon rapprocher UID et indicateurs par lots, suppressions comprises (un message supprimé depuis un autre appareil disparaît de la projection).
- Détection d'une nouvelle UIDVALIDITY à la sélection d'un dossier : invalider la correspondance distante du dossier et le reconstruire en priorité.
- Rapprochement des identités par Message-ID, date et taille : quand il est certain, conserver corps en cache, décisions du tri, destination ponctuelle et marques de piles ; sinon ne rien rattacher.
- Opérations en attente portant sur d'anciens identifiants : état `a_verifier`. Elles ne sont jamais appliquées à un message ayant repris le même UID ; elles sont rattachées si le rapprochement est certain, sinon présentées à l'utilisateur.
- Republier la vue (nouvelle révision) seulement après reconstruction.
- Tests : provoquer une nouvelle génération sur le banc (suppression puis recréation du dossier). Si le banc GreenMail ne le permet pas, utiliser un conteneur Dovecot ajouté à `bench/` et le documenter.

## Critères d'acceptation
- [ ] Après une nouvelle UIDVALIDITY, le dossier est reconstruit sans doublon et les décisions du tri sont conservées.
- [ ] Une opération en attente n'atteint jamais un autre message ayant repris le même UID.
- [ ] Une suppression faite depuis un autre client disparaît à la resynchronisation.
- [ ] Un message arrivé pendant un déplacement n'est ni perdu ni dupliqué.

## Vérification
```
make check
make test-integration
```

## Dépendances
« Traiter une action optimiste : étoile, lu, non lu », « Rattraper une interruption du flux d'événements ».