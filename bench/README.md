# Banc de courrier GreenMail

Depuis la racine du dépôt, avec Docker Compose, Python 3.11 ou ultérieur et GNU Make, lancez le banc puis peuplez-le.

```sh
make bench-up
make bench-seed DATASET=base
# Utiliser la boîte, puis arrêter le banc
make bench-down
```

`bench-up` attend la bannière IMAP. `bench-seed` recrée le conteneur du projet Compose avant chaque import. Cette commande remplace tous les messages et dossiers des trois comptes, y compris ceux ajoutés à la main. `bench-down` supprime le conteneur et son réseau. GreenMail conserve les boîtes en mémoire.

## Se connecter

Les quatre ports sont publiés sur `127.0.0.1`. Les deux ports TLS utilisent le certificat de test de GreenMail.

| Protocole | Port local | Variable pour changer le port |
| --- | ---: | --- |
| IMAP | 3143 | `PLI_BENCH_IMAP_PORT` |
| IMAPS | 3993 | `PLI_BENCH_IMAPS_PORT` |
| SMTP | 3025 | `PLI_BENCH_SMTP_PORT` |
| SMTPS | 3465 | `PLI_BENCH_SMTPS_PORT` |

| Identifiant de connexion | Mot de passe fictif |
| --- | --- |
| `alice@recette.test` | `alice-test` |
| `bob@recette.test` | `bob-test` |
| `carol@recette.test` | `carol-test` |

## Choisir les messages

| `DATASET` | Messages dans INBOX d’Alice | Usage |
| --- | ---: | --- |
| `vide` | 0 | Premier rattachement à une boîte vide |
| `base` | 20 | Lecture, tri, pièces jointes et intégration continue |
| `10k` | 10 000 | Import et pagination en local |
| `100k` | 100 000 | Indexation et recherche en local |

Bob et Carol commencent avec une boîte vide pour chaque jeu. Les jeux peuplés créent aussi trois dossiers vides chez Alice. `Atelier` est personnel, `Gesendet` représente les Envoyés et `Papierkorb` la Corbeille.

La graine par défaut est `42`. Par exemple, `make bench-seed DATASET=10k SEED=7` reproduit le même ordre, les mêmes octets MIME, dates, liens de fil et indicateurs de lecture à chaque exécution. Les UID des messages repartent de 1. GreenMail attribue une nouvelle génération IMAP (`UIDVALIDITY`) à la boîte recréée.

Les jeux de 10 000 et 100 000 messages prolongent le jeu de base avec des conversations de 25 messages. Sur le jeu `100k`, le comptage IMAP a confirmé les 100 000 messages. Avec GreenMail 2.1.15, un `FETCH` de l’en-tête du dernier message a pris plus de quatre minutes. Consignez ce temps fournisseur séparément lors des mesures.

| Position dans INBOX | Cas de recette |
| --- | --- |
| 1 | Premier message d’un nouvel expéditeur, à trier |
| 2 et 3 | Message déjà lu, puis réponse non lue liée par `In-Reply-To` et `References` |
| 4 | Lettre d’information avec `List-ID` et lien de désinscription |
| 5 | Reçu avec le PDF de référence [recu.pdf](fixtures/recu.pdf) |
| 6 | Versions texte et HTML, timbre PNG joint par `cid:` et image distante sur `images.recette.test` |
| 7 | Objet et texte avec accents, caractères japonais, grecs et cyrilliques |
| 8 et 9 | Même en-tête `Date` et même `INTERNALDATE` IMAP |
| 10 et 11 | Même objet, identifiants distincts et aucune référence de fil |
| 12 | Pièce jointe de 2 Mio pour les essais de téléchargement |
| 13 et suivants | Conversations de l’atelier pour la lecture et la recherche |

## Capacités après authentification

Échange relevé sur le port IMAP de `greenmail/standalone:2.1.15` le 27 novembre 2026.

```text
a1 LOGIN alice@recette.test alice-test
a1 OK LOGIN completed.
a2 CAPABILITY
* CAPABILITY IMAP4rev1 LITERAL+ UIDPLUS SORT IDLE MOVE SASL-IR AUTH=XOAUTH2 QUOTA
a2 OK CAPABILITY completed.
```

| Extension | Annoncée | Extension | Annoncée |
| --- | --- | --- | --- |
| IDLE | Oui | CONDSTORE | Non |
| MOVE | Oui | QRESYNC | Non |
| UIDPLUS | Oui | SPECIAL-USE | Non |

## Tester l’intégration

`make test-integration` vérifie les fixtures, peuple le jeu de base, lance les tests Rust marqués puis supprime son projet Compose, y compris si un test échoue. Docker attribue ses quatre ports pour permettre l’exécution pendant une recette manuelle. Pour ajouter un test, préfixez son nom avec `greenmail_`, marquez-le `#[ignore = "requiert le banc GreenMail"]` et lisez le port IMAP dans `PLI_BENCH_IMAP_PORT`. Le job CI `integration` exécute cette cible.

## Écrire une trace de mesure

Dans `bench/traces/`, un fichier `.jsonl` contient un objet JSON UTF-8 par ligne, pour une répétition d’un parcours. Le fichier [exemple.jsonl](traces/exemple.jsonl) illustre une ouverture en cache et un archivage avec des valeurs fictives.

| Champ | Valeur attendue |
| --- | --- |
| `version`, `parcours` | Entier `1` et nom stable du parcours, par exemple `ouvrir_fil` ou `archiver`. |
| `origine_utc` | Instant UTC au format RFC 3339 correspondant au zéro de l’horloge monotone du client. |
| `clic` | Horodatage de l’action, en millisecondes depuis cette origine. |
| `rendu_contenu` | Horodatage où le contenu visé devient lisible ou la modification visible. Un squelette de chargement garde la mesure ouverte. |
| `confirmation_distante` | Horodatage où le client reçoit la confirmation durable de l’action. Une lecture en cache utilise `null`. |
| `conditions` | Jeu et graine, état du cache (`chaud`, `froid_serveur_pret` ou `premier_rattachement`), fournisseur et version, navigateur et version, appareil, fenêtre en pixels, débits et aller-retour réseau. |
| `phase`, `resultat` | `echauffement` ou `mesure`, puis `ok` ou `erreur`. Pour un échec, ajouter un `code_erreur` stable et mettre à `null` chaque étape non observée. |

Les trois horodatages partagent la même horloge et acceptent les fractions de milliseconde. Calculez la latence visible par `rendu_contenu - clic` et la latence confirmée par `confirmation_distante - clic`. Si le parcours les mesure, ajoutez dans `mesures` les durées `cpu_navigateur_ms`, `serveur_ms`, `fournisseur_ms` et `attente_reseau_ms`. Conservez uniquement des conditions anonymisées et des codes techniques dans les traces.
