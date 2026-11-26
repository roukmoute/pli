---
id: k6jpv3k35
title: Tester la synchronisation par injection de pannes
depends_on: k7pfstynd, kmqbjnt1y, khf35f65q, kd8focfhy, ks5rethb9, kqhbe3h9k, kwwlmwlbk
---

## Contexte
Webmail Infomaniak à interface HEY (back Rust dans `server/`, front dans `web/`, banc dans `bench/`, Playwright dans `e2e/`). Cahier : `Kanban/.specs/piece-TlW9Y4/cahier-des-charges-webmail-infomaniak-hey.md`, paragraphe « Essais de fiabilité ». La perte de brouillon, le double envoi et la fuite intercompte sont des anomalies bloquantes.

## À faire
- Ajouter au banc un proxy de pannes entre le serveur du webmail et le serveur de courrier (Toxiproxy dans Docker, version vérifiée), et une cible `make test-pannes`.
- Coupure avant puis après l'acceptation d'une action de classement : l'état final est juste et l'opération n'est appliquée qu'une fois.
- Événement SSE double puis manquant ; expiration de session pendant une action.
- Dossier supprimé depuis un autre appareil pendant un déplacement en attente.
- Arrivée d'un message pendant une pagination ; index de recherche en reconstruction.
- Disque plein côté serveur (répertoire de données sur un petit volume `tmpfs`) : aucune corruption, erreur claire.
- Stockage navigateur évincé ; très gros HTML ; fichier dépassant la limite.
- Tri : premier message d'un expéditeur reçu pendant l'import, deux arrivées de la même adresse, confirmation concurrente d'un refus. Un message en attente n'apparaît dans aucune notification ordinaire, galerie ni recherche implicite.
- Corriger les défauts mineurs ; lister les autres en fin de journal de carte.

## Critères d'acceptation
- [ ] `make test-pannes` passe et nettoie le banc.
- [ ] Chaque panne de la liste a au moins un test nommé.
- [ ] Aucun test ne révèle de perte de brouillon, de double application ni de donnée d'un autre compte.
- [ ] La suite est documentée dans `bench/README.md`.

## Vérification
```
make test-pannes
make check
```

## Dépendances
« Tester le jalon Lecture de bout en bout avec Playwright », « Rattraper une interruption du flux d'événements », « Reconstruire un dossier après changement d'UIDVALIDITY », « Gérer les limites de stockage et les reprises ».