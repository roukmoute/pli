---
id: k700b90hs
title: Décider du cache partagé et du hub temps réel
depends_on: kmqbjnt1y, krsvz68mw
---

## Contexte
Webmail Infomaniak à interface HEY (back Rust/Axum dans `server/`, flux SSE direct et journal de révisions, banc de mesure `make perf` et rapports dans `docs/mesures/`). Cahier : `Kanban/.specs/piece-TlW9Y4/cahier-des-charges-webmail-infomaniak-hey.md`, carte S08, ligne « Redis et Mercure » du tableau des caches. Le cahier demande une décision fondée sur un essai, pas une adoption par défaut.

## À faire
- Mesurer la référence : une instance, SSE direct d'Axum, avec un nombre de connexions simultanées représentatif d'une instance personnelle à plusieurs boîtes (hypothèse à écrire dans le rapport).
- Essai Mercure : hub Mercure dans Docker (version vérifiée), abonnements privés par compte, publication des révisions par le service Rust derrière une abstraction de diffusion. Même débit, même mesure du p95 « notification vers objet visible ».
- Essai Redis : cache de lectures coûteuses avec clés par compte et version, et diffusion entre deux instances du service. Rappel : Redis Pub/Sub distribue au plus une fois ; la révision durable doit assurer le rattrapage.
- Dans chaque essai, tester la portée privée (un compte ne reçoit rien d'un autre) et la reprise après un événement manqué.
- Rédiger `docs/decisions/cache-partage-et-temps-reel.md` : chiffres avec et sans chaque composant, coût d'exploitation, décision motivée (maintien du service unique, ou adoption).
- Le code d'essai reste derrière une option désactivée par défaut, ou est retiré si la décision est le maintien.

## Critères d'acceptation
- [ ] Le document compare les trois configurations au même débit et au même p95.
- [ ] La portée privée et la reprise sont testées dans chaque essai.
- [ ] La décision est explicite et datée.
- [ ] Le comportement par défaut du produit est inchangé et `make check` passe.

## Vérification
```
make check
make test-integration
```

## À vérifier par l'humain
- Valider la décision proposée.

## Dépendances
« Rattraper une interruption du flux d'événements », « Mesurer les budgets de latence ».