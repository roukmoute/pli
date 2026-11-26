---
id: krsvz68mw
title: Mesurer les budgets de latence
depends_on: k7pfstynd, kirtkkako, kh8stj2yu, k9g91mi7z, ks5rethb9, kqhbe3h9k
---

## Contexte
Webmail Infomaniak à interface HEY (banc dans `bench/` avec jeux `base`, `10k`, `100k`, format de trace dans `bench/traces/`, Playwright dans `e2e/`). Cahier : `Kanban/.specs/piece-TlW9Y4/cahier-des-charges-webmail-infomaniak-hey.md`, carte S07, section « Mesure des performances et recette ». Les cibles du cahier restent à démontrer sur le produit.

## À faire
- Banc de mesure `e2e/perf/` lancé par `make perf` (hors CI) : Chromium sans écran, profil réseau imposé (10 Mbit/s en réception, 2 Mbit/s en envoi, 80 ms d'aller-retour).
- Mesurer les sept parcours du tableau du cahier : retour du fil à la Réception ; ouverture d'un fil avec corps en cache ; archivage ; première page lisible caches vides ; recherche sur 100 000 messages ; notification IMAP jusqu'à l'objet visible ; acceptation d'un expéditeur.
- La mesure s'arrête au contenu réellement visible : un squelette de chargement ne termine aucune mesure. Chronométrer séparément la réaction locale et la confirmation distante.
- Au moins 100 répétitions par parcours après une phase d'échauffement distincte.
- Distinguer les états chaud, froid avec serveur prêt, et premier rattachement (avec progression avant indexation complète).
- Rapport `docs/mesures/rapport-AAAA-MM-JJ.md` : médiane, p95, taux d'erreur, volume transféré, nombre de requêtes, taux de cache utile, versions figées, matériel, conditions ; pour chaque budget, atteint ou non. Les traces des cas lents sont conservées sans donnée privée.

## Critères d'acceptation
- [ ] `make perf` produit le rapport sans intervention.
- [ ] Chaque parcours a médiane, p95, erreurs et conditions.
- [ ] Le rapport dit clairement quels budgets sont atteints ; un budget manqué ne fait pas échouer la carte mais est listé avec sa cause probable.
- [ ] Temps navigateur, temps serveur et attente réseau sont séparés.

## Vérification
```
make perf
make check
```

## À vérifier par l'humain
- Rejouer deux parcours sur un téléphone de milieu de gamme.

## Dépendances
« Tester le jalon Lecture de bout en bout avec Playwright », « Traiter une action optimiste : étoile, lu, non lu », « Recevoir les changements en direct par IDLE et SSE », « Mettre vues et fils récents en cache IndexedDB », « Archiver une sélection et garder la place de lecture », « Chercher dans le dossier actif avec FTS5 ».