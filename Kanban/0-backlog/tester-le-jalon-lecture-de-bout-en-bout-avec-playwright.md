---
id: k7pfstynd
title: Tester le jalon Lecture de bout en bout avec Playwright
depends_on: ks1io8wuk, kx7bmitqo
---

## Contexte
Webmail Infomaniak à interface HEY (back Rust dans `server/`, front SolidJS dans `web/`, banc GreenMail dans `bench/`, CI dans `.github/workflows/ci.yml`). Cahier : `Kanban/.specs/piece-TlW9Y4/cahier-des-charges-webmail-infomaniak-hey.md`, section « Jalons et critères de livraison » (jalon Lecture) et tableau « Mesure des performances ». Le jalon Lecture est codé ; cette carte installe les tests de bout en bout dans un navigateur sans écran et rejoue le scénario complet.

## À faire
- Installer Playwright 1.64 (vérifier la version courante) dans `e2e/`, Chromium sans écran. `make e2e` démarre le banc, peuple le jeu `base`, construit et lance le serveur sur une base vierge, lance les tests, puis arrête tout.
- Utilitaires de test : connexion d'un compte du banc, envoi d'un message par SMTP au banc, redémarrage du serveur.
- Scénario du jalon : rattacher la boîte d'Alice ; consulter l'historique pendant l'import ; recevoir deux messages d'un nouvel expéditeur ; constater une seule entrée au Screener et leur absence de la Réception ; redémarrer ; accepter vers Réception ; ouvrir le fil ; revenir à la liste. Puis refuser un autre expéditeur, redémarrer et vérifier que les deux décisions ont survécu.
- Même retour à la liste en fenêtre 390 × 844.
- Mesure : retour du fil à la Réception, 100 répétitions après une phase d'échauffement, du clic à la première ligne visible. Le test échoue si le p95 atteint 50 ms ou si une requête réseau part avant l'affichage. Écrire la trace au format de `bench/traces/`.
- Ajouter un job `e2e` à la CI.

## Critères d'acceptation
- [ ] `make e2e` passe en local, sans écran, et nettoie le banc.
- [ ] Le scénario couvre import, tri, redémarrage, acceptation, refus et lecture.
- [ ] La mesure produit médiane et p95 dans une trace.
- [ ] Le job CI existe et le workflow reste valide.

## Vérification
```
make e2e
make check
docker run --rm -v "$PWD":/repo -w /repo rhysd/actionlint:latest
```

## À vérifier par l'humain
- Même parcours sur un vrai téléphone.

## Dépendances
« Mettre en place la CI GitHub Actions », « Ouvrir un message assaini et retrouver la liste ».