---
id: kvogo9if0
title: Gérer les piles « Répondre plus tard » et « De côté »
depends_on: kirtkkako, kmqbjnt1y
---

## Contexte
Webmail Infomaniak à interface HEY (back Rust dans `server/`, front SolidJS dans `web/`, opérations idempotentes, flux SSE et rattrapage). Cahier : `Kanban/.specs/piece-TlW9Y4/cahier-des-charges-webmail-infomaniak-hey.md`, carte L10, ligne « Piles au bas de la réception », ligne « Conversations et piles » du tableau des données.

## À faire
- Migration `thread_marks` : marque `repondre_plus_tard` ou `de_cote` posée sur un fil du compte. Ces marques sont propres à l'application : ni le dossier IMAP ni l'étoile ne changent.
- Opérations `marquer_pile` et `retirer_pile`, avec révision et événements.
- API : contenu des deux piles, et leurs compteurs dans la réponse de la vue Réception.
- Front : deux piles au bas de la Réception, repliées par défaut ; chacune se déplie et mène au fil choisi. Les tuiles du menu ouvrent la pile en pleine page.
- Dans le fil : les boutons « Répondre plus tard » et « Mettre de côté » posent ou retirent la marque.
- Les piles se parcourent au clavier, y compris repliées.

## Critères d'acceptation
- [ ] Ajouter deux fils à Répondre plus tard et un à De côté : les piles affichent 2 et 1.
- [ ] Déplier une pile puis choisir un fil ouvre le bon.
- [ ] Retirer une marque depuis un second onglet met à jour le premier sans rechargement.
- [ ] Le dossier IMAP et l'étoile des fils marqués sont inchangés (vérification IMAP).
- [ ] Les marques survivent à un redémarrage.

## Vérification
```
make check
make test-integration
```

## Dépendances
« Traiter une action optimiste : étoile, lu, non lu », « Rattraper une interruption du flux d'événements ».