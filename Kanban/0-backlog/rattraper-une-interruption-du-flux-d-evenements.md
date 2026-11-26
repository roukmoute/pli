---
id: kmqbjnt1y
title: "Rattraper une interruption du flux d'événements"
depends_on: kh8stj2yu, k9g91mi7z
---

## Contexte
Webmail Infomaniak à interface HEY (back Rust dans `server/`, front dans `web/`, module `web/src/sync/` avec `evenements.ts` et `cache.ts`). Cahier : `Kanban/.specs/piece-TlW9Y4/cahier-des-charges-webmail-infomaniak-hey.md`, carte S03, paragraphes « Contrat entre front et back » et « Reprise après incident ». Le flux SSE existe, mais un événement perdu ou reçu deux fois peut laisser l'onglet dans un état faux.

## À faire
- Back : journal durable des changements par compte et par révision, avec une rétention bornée (nombre de révisions configurable). `GET /api/v1/changes?depuis=` répond `410 revision_expiree` quand la révision demandée n'est plus dans le journal.
- Front : mémoriser la dernière révision appliquée (mémoire et cache IndexedDB). À la reconnexion SSE et à la réouverture d'un onglet, demander les changements depuis cette révision.
- Sur `410`, reprendre un instantané de la vue active et invalider les vues en cache.
- Application idempotente : un événement dont la révision est déjà appliquée ne change rien ; un trou de révision déclenche un rattrapage.
- Reconnexion SSE avec délai croissant et indicateur discret « hors ligne ».
- Avant de rejouer des opérations en attente à la réouverture, comparer la révision locale à celle du serveur.

## Critères d'acceptation
- [ ] Un événement perdu puis un événement dupliqué conduisent à la même révision et au même état qu'un flux sans incident.
- [ ] Une révision trop ancienne déclenche un instantané du dossier.
- [ ] Deux onglets se retrouvent sur la même révision après coupure.
- [ ] Un événement reçu deux fois ne modifie pas les compteurs.

## Vérification
```
make check
make test-integration
```

## Dépendances
« Recevoir les changements en direct par IDLE et SSE », « Mettre vues et fils récents en cache IndexedDB ».