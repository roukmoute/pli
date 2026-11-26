---
id: k0ptcluj1
title: Préparer le banc GreenMail et ses messages fictifs
depends_on: kld8ie51k, ks1io8wuk, kmu2pmyh6
---

## Contexte
Webmail Infomaniak à interface HEY (back Rust dans `server/`, front dans `web/`, `Makefile`, `README.md`). Cahier : `Kanban/.specs/piece-TlW9Y4/cahier-des-charges-webmail-infomaniak-hey.md`, carte F02 et section « Mesure des performances et recette ». Les agents n'ont pas de boîte Infomaniak : toutes les cartes sont vérifiées sur un serveur IMAP/SMTP local dans Docker. Cette carte le fournit.

## À faire
- `bench/docker-compose.yml` : GreenMail (`greenmail/standalone`, tag 2.1.x exact vérifié sur Docker Hub) exposant IMAP, IMAPS, SMTP et SMTPS sur des ports locaux documentés. Comptes `alice@recette.test`, `bob@recette.test`, `carol@recette.test` avec mots de passe de test.
- Outil de peuplement déterministe (binaire Rust `bench/seed` ou équivalent, même graine = mêmes messages) avec les jeux `vide`, `base`, `10k` et `100k`. Le jeu `base` (20 messages pour alice) contient : un nouvel expéditeur, une réponse dans un fil déjà lu, une lettre d'information, un reçu, un PDF joint, un message HTML avec image intégrée et image distante, des accents et caractères non latins, deux messages à date identique, deux messages indépendants de même objet, un dossier personnel, et des dossiers Envoyés/Corbeille aux libellés étrangers.
- Cibles `make bench-up`, `make bench-down`, `make bench-seed DATASET=base`, `make test-integration` (démarre le banc, peuple, lance les tests Rust d'intégration marqués, arrête le banc).
- Un premier test d'intégration : connexion IMAP d'alice et comptage des messages du jeu `base`.
- `bench/README.md` : ports, comptes, jeux, et la réponse `CAPABILITY` réelle du banc après authentification (IDLE, CONDSTORE, QRESYNC, SPECIAL-USE, MOVE, UIDPLUS présents ou non). Les cartes suivantes s'appuient sur ce relevé.
- Trace de mesure : définir le format JSONL de `bench/traces/` (parcours, horodatages `clic`, `rendu_contenu`, `confirmation_distante`, conditions) et le documenter.
- Ajouter un job `integration` à `.github/workflows/ci.yml` qui lance `make test-integration`.

## Critères d'acceptation
- [ ] `make bench-up && make bench-seed DATASET=base` donne une boîte identique d'un lancement à l'autre.
- [ ] `make test-integration` passe et laisse le banc arrêté.
- [ ] Le relevé de capacités est écrit dans `bench/README.md`.
- [ ] Les jeux `10k` et `100k` se génèrent (non lancés en CI).

## Vérification
```
make test-integration
make bench-up && make bench-seed DATASET=base && make bench-down
make check
```

## Dépendances
« Mettre en place les tests », « Mettre en place la CI GitHub Actions ».