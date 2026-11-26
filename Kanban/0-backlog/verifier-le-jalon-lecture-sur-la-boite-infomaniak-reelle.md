---
id: kbale6oum
title: Vérifier le jalon Lecture sur la boîte Infomaniak réelle
depends_on: kx7bmitqo
---

## Contexte
Webmail Infomaniak à interface HEY (back Rust dans `server/`). Cahier : `Kanban/.specs/piece-TlW9Y4/cahier-des-charges-webmail-infomaniak-hey.md`, paragraphes « Infomaniak » et « Arbitrages encore ouverts ». Tout a été vérifié sur un banc local ; cette carte confronte le jalon Lecture à une vraie boîte Infomaniak dédiée aux essais, **en lecture seule**.

## Prérequis fourni par l'humain
Le fichier `.env.recette` à la racine (ignoré par git) contient `RECETTE_IMAP_USER` et `RECETTE_IMAP_PASSWORD`. **S'il est absent, arrête-toi et signale-le dans le journal : ne simule rien.** N'écris jamais ces valeurs dans un fichier suivi, un journal ou une commande affichée.

## À faire
- Créer `make recette-lecture` : charge `.env.recette`, lance le serveur sur une base temporaire avec le préréglage Infomaniak (`mail.infomaniak.com`, 993 TLS implicite) et exécute un programme de vérification.
- Vérifier : authentification ; capacités annoncées après connexion (IDLE, CONDSTORE, QRESYNC, SPECIAL-USE, MOVE, UIDPLUS) ; découverte des dossiers et de leurs rôles ; repère INBOX ; import des en-têtes ; première page de Réception ; lecture d'un corps texte, d'un corps HTML et de la liste des pièces.
- Contrainte : dossiers ouverts par `EXAMINE`, lectures par `BODY.PEEK`. Aucun envoi, déplacement, suppression ni changement d'indicateur. Le marquage lu à l'ouverture est désactivé pour ce programme. Vérifier que le nombre de messages non lus est identique avant et après.
- Écrire `docs/recette/infomaniak-lecture.md` : date, capacités relevées, rôles reconnus ou non, durées d'import, écarts constatés avec le banc. Aucune adresse complète, aucun objet, aucun contenu.
- Corriger les écarts simples (libellé de dossier non reconnu…) ; lister les autres dans le rapport.
- Cette commande ne tourne jamais en CI.

## Critères d'acceptation
- [ ] `make recette-lecture` se termine avec un résultat par étape.
- [ ] Le compte de non-lus de la boîte est inchangé.
- [ ] Le rapport existe, sans donnée privée.
- [ ] `git status` ne montre aucun secret.

## Vérification
```
make recette-lecture
make check
git status --short
```

## Dépendances
« Ouvrir un message assaini et retrouver la liste ».