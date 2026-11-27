# Lecture d’une boîte Infomaniak

Le 27 novembre 2026, Pli a rattaché une boîte d’essai par IMAP sur TLS, affiché sa première page de Réception et lu des messages texte et HTML. Pour une pièce jointe, le nom décodé, le type MIME et la taille rendus par l’API correspondaient au message source. Deux relevés IMAP indépendants ont confirmé le même nombre de non-lus dans chacun des dossiers sélectionnables, y compris la Réception.

## Compatibilité observée

| Vérification | Boîte réelle | Banc local GreenMail |
| --- | --- | --- |
| Authentification IMAP et ouverture d’une session Pli | Réussies | Réussies |
| Capacités annoncées après connexion | IDLE, CONDSTORE, QRESYNC, SPECIAL-USE, MOVE, UIDPLUS | IDLE, MOVE, UIDPLUS |
| Rôles reconnus par Pli | Réception, Envoyés, Brouillons, Indésirables, Corbeille, Archives | Réception, Envoyés, Corbeille, Archives |
| Génération et prochain UID enregistrés au rattachement | Identiques au relevé IMAP | Identiques au relevé IMAP |
| En-têtes indexés en Réception | Total conforme au relevé initial | Total conforme au jeu de 20 messages |
| Variation du nombre de non-lus | 0 dans chaque dossier | 0 dans chaque dossier |

La boîte réelle annonce `SPECIAL-USE` et omet l’attribut `\Archive` sur son dossier d’archives. Pli reconnaît désormais un dossier sélectionnable au libellé standard, à la racine ou directement sous INBOX, lorsque le choix est unique. Une attribution explicite du serveur prime. En présence de plusieurs candidats, Pli conserve leur classement d’origine.

## Durées mesurées

| Opération | Boîte réelle | Banc local |
| --- | ---: | ---: |
| Première page et en-têtes du premier fil disponibles | 2,66 s | 1,21 s |
| Import complet des en-têtes de Réception | 54,38 s | 1,21 s |
| Lecture à froid d’un message texte | 485 ms | 325 ms |
| Lecture à froid d’un message HTML | 476 ms | 350 ms |
| Lecture à froid avec inventaire des pièces | 492 ms | 350 ms |

Chaque essai utilise le binaire de production et une base SQLite temporaire vide. Les deux premiers temps partent du début du rattachement et incluent la vérification de disponibilité par intervalles de 200 ms. Les lectures à froid couvrent une requête HTTP locale, le parcours IMAP et le rendu côté serveur. Le banc et la boîte distante ont des volumes et des conditions réseau différents. Ces durées décrivent une exécution sur chaque environnement.

## Rejouer l’essai

Renseigner les variables `RECETTE_IMAP_USER` et `RECETTE_IMAP_PASSWORD` dans le fichier local `.env.recette` à la racine. Le fichier est ignoré par Git. La commande utilise Python 3 avec SQLite et le préréglage `mail.infomaniak.com:993` en TLS implicite.

```sh
make recette-lecture
```

La commande affiche un résultat pour chaque étape. Elle ouvre les dossiers par `EXAMINE`, lit les corps par `BODY.PEEK[]` et consulte l’API de contenu de Pli. Le marquage lu de l’interface nécessite une action distincte. Le relevé final des non-lus s’exécute également si la lecture d’un échantillon échoue. La base temporaire est supprimée à la sortie. L’accès à la boîte réelle est réservé à l’exécution manuelle.

Pour reproduire la mesure du banc avec le même programme et le jeu fictif `base`, utiliser la commande suivante après la construction du binaire de production.

```sh
bash bench/avec-greenmail.sh "$(command -p -v python3 || command -v python3)" -B bench/recette_lecture.py server/target/release/webmail-server --banc
```
