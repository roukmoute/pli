# Installer et développer Pli

## Démarrer Pli

Depuis la racine du dépôt, avec rustup, un compilateur C, GNU Make et Node 24 muni de npm, lancez `make dev`. Si vous utilisez nvm, `nvm install` sélectionne la version du fichier `.nvmrc`. Rustup installe la chaîne du projet au premier appel à Cargo. Le lancement en développement utilise Bash 4.3 ou ultérieur.

Ouvrez <http://127.0.0.1:5173>. L’accueil permet de se connecter à une boîte et restaure la Réception quand la session est encore valide. Vite transmet les requêtes `/api` à Axum. Ctrl+C arrête les deux processus.

| Commande | Résultat |
| --- | --- |
| `make dev` | Lance le serveur Rust et Vite avec rechargement du front. |
| `make lint` | Vérifie le format Rust, lance Clippy sur toutes les cibles avec les avertissements en erreur, puis Biome et TypeScript en mode strict. |
| `make build` | Construit `web/dist/`, puis le binaire `server/target/release/webmail-server`. |
| `make test` | Exécute les tests Rust et les tests du front une fois. |
| `make check` | Lance le lint, compile le projet et exécute les deux suites de tests. |
| `make test-integration` | Peuple un GreenMail temporaire, exécute les tests d’intégration de messagerie puis arrête le banc. |

Les commandes qui utilisent le front installent ses dépendances avec `npm ci` au premier lancement et lorsque le manifeste ou son verrou change. Après `make build`, lancez le binaire depuis la racine. L’API et l’accueil sont alors accessibles sur le même port.

```sh
./server/target/release/webmail-server
```

Dans un second terminal, `curl -s http://127.0.0.1:8080/api/v1/health` renvoie `{"status":"ok"}` si la lecture SQLite réussit.

## Intégration continue

À chaque push et à chaque pull request, le job `check` lance `make lint`, `make build` puis `make test` sur Ubuntu avec les versions de Rust et de Node du dépôt. Il fonctionne sans secret de messagerie.

Le job `integration` du même workflow vérifie le jeu de 20 messages avec Docker. Pour démarrer le banc à la main, choisir un jeu ou consulter les capacités IMAP mesurées, suivez le [guide du banc GreenMail](../bench/README.md).

## Écrire et lancer un test

`scripts/qualite.sh mutation` affiche le tag de départ et rejoue chaque site sélectionné. Les tests d’intégration utilisent un GreenMail jetable. Une équivalence documentée vaut pour une empreinte du fichier, une position et un remplacement précis. Si le fichier change, toute mutation encore survivante demande une nouvelle revue. Le code de sortie 2 signale un témoin en échec et le code 3 un survivant sans justification. [Équivalences examinées](../scripts/mutations-equivalentes.toml)

**Rust.** Ajoutez un `#[test]` au module `tests` du fichier concerné. Pour tester une route, utilisez `#[tokio::test]`. L’instance créée par `let app = crate::test_support::TestApp::new()?;` conserve la base SQLite temporaire migrée, accessible par `app.db`, pendant toute sa portée. Importez `tower::ServiceExt` pour appeler le routeur en mémoire avec `app.router().oneshot(request).await?`. [Exemple de test HTTP](../server/src/http.rs)

**Front.** Placez un fichier `*.test.tsx` à côté du composant ou `*.test.ts` à côté du module. Montez un composant avec `render` et attendez le texte affiché avec `findByText`. Pour le client HTTP, simulez `fetch` avec `vi.stubGlobal`. Vitest utilise jsdom, nettoie les composants montés et restaure les variables globales simulées entre les tests. [Test du composant](../web/src/App.test.tsx) · [Test du client HTTP](../web/src/sync/api.test.ts)

Depuis la racine, `make test` prépare les dépendances et lance les deux suites. Pour cibler une suite ou un fichier, utilisez les commandes suivantes.

| Suite | Toute la suite | Un seul fichier |
| --- | --- | --- |
| Rust | `cargo test --manifest-path server/Cargo.toml --locked` | `cargo test --manifest-path server/Cargo.toml --locked db::tests::` pour `server/src/db.rs` |
| Front | `npm --prefix web test` | `npm --prefix web test -- src/sync/api.test.ts` |

## Configuration

| Variable d’environnement | Défaut | Usage |
| --- | --- | --- |
| `WEBMAIL_LISTEN` | `127.0.0.1:8080` | Adresse d’écoute du serveur et cible du proxy de développement. |
| `WEBMAIL_DATA_DIR` | `./data` | Répertoire local qui accueille `webmail.db` et les fichiers WAL. Il est créé au démarrage. |
| `WEBMAIL_WEB_DIR` | `web/dist` | Répertoire du front de production servi par Axum, y compris à l’ouverture directe d’une vue. |
| `WEBMAIL_ADMIN_TOKEN` | vide | Jeton à saisir sur `/admin`. L’accès à l’administration exige un jeton configuré côté serveur. |
| `WEBMAIL_ALLOW_TEST_PROFILE` | `0` | La valeur `1` autorise le profil GreenMail local avec certificat de test sur une IP de bouclage. |
| `WEBMAIL_SECRET_KEY` | vide | Clé de 32 octets encodée en 64 chiffres hexadécimaux, requise pour ouvrir une session de courrier. Générez-la une fois avec `openssl rand -hex 32` et conservez-la dans la configuration privée de l’instance. |
| `WEBMAIL_SESSION_TTL_SECONDS` | `28800` | Durée d’une session en secondes à compter de la connexion, soit huit heures par défaut. |
| `WEBMAIL_PAGE_SIZE` | `20` | Nombre de fils d’une page de liste, de 1 à 100, quand la requête ne précise pas `taille`. L’import IMAP garde ses lots de 50 en-têtes. |

Sur `/admin`, ouvrez l’administration puis choisissez **Ajouter un serveur**. Renseignez les hôtes IMAP et SMTP, les ports, les modes TLS et les domaines de votre service de messagerie. Le banc local propose IMAPS 3993 et SMTPS 3465. Les ports sont modifiables.

**Tester** affiche la résolution DNS, la connexion TCP, la négociation TLS et l’authentification pour chaque protocole. **Tester et enregistrer** applique le profil après la réussite des deux authentifications. Les identifiants servent uniquement à cette requête. Le diagnostic ne lit ni n’envoie de message.

L’API d’administration utilise l’en-tête `Authorization: Bearer <jeton>`. `GET /api/admin/v1/servers` renvoie les serveurs et les préréglages. `POST /api/admin/v1/servers/test` teste un candidat. `POST /api/admin/v1/servers` le crée et `PUT /api/admin/v1/servers/{id}` le modifie. Ces trois opérations reçoivent un objet JSON contenant `profile` (même structure qu’un préréglage) et `credentials` (champs `username` et `password`). Une sauvegarde refusée renvoie le diagnostic avec le statut HTTP 422 et conserve le profil en service.

Les scénarios de `scripts/acceptation.sh` démarrent un GreenMail Docker temporaire avec le jeu `base`. Pour réutiliser un banc déjà peuplé avec ce jeu, exportez ses deux ports dans `PLI_APS_IMAPS_PORT` et `PLI_APS_SMTPS_PORT`. Les scénarios utilisent les boîtes fictives d’Alice et de Bob du [banc GreenMail](../bench/README.md).

Make génère `.env.example` à partir de `config/example.env`. La commande `make .env.example` permet aussi de le préparer seul. Pour utiliser vos valeurs dans le terminal courant, copiez le modèle dans `.env`, modifiez-le, puis exportez ses variables avant de lancer le service.

```sh
set -a
. ./.env
set +a
```

## Connecter une boîte

Sur `/admin`, associez le domaine des adresses au profil du fournisseur. Pour une connexion, un seul profil actif doit mentionner ce domaine. Pli vérifie ensuite l’adresse complète et le secret auprès de son serveur IMAP, sous TLS. Au premier succès, il crée le profil utilisateur et la boîte rattachée. Les connexions suivantes retrouvent la même boîte. Pour la recette locale, lancez `make bench-up`, activez le profil de test et enregistrez « Banc local » pour `recette.test`.

| Méthode | Opération sur `/api/v1/session` |
| --- | --- |
| `POST` | Reçoit `{"address":"alice@recette.test","password":"alice-test"}` (exemple du banc). Renvoie `account_id`, `address` et `expires_at` (secondes Unix) et pose le cookie de session. Un refus d’identification donne `401 identifiants_invalides`. Une panne IMAP, TLS ou de stockage donne `503 service_indisponible`. |
| `GET` | Renvoie la boîte attachée au cookie. Une session absente, expirée ou fermée donne `401 session_expiree`. |
| `DELETE` | Révoque la session en base, efface le cookie et renvoie `204`. |

Au premier rattachement, la session s’ouvre une fois les dossiers et le repère INBOX enregistrés ensemble. Le repère contient la génération IMAP et le prochain UID relevés avant l’import. Chaque compte conserve ce repère lors des connexions suivantes et des redémarrages.

Lorsque le serveur annonce `SPECIAL-USE`, ses attributs déterminent les rôles des dossiers. Sinon, le synchroniseur reconnaît les libellés français, allemands et anglais à la racine ou directement sous INBOX. Un sous-dossier personnel tel que `Atelier/Archives` garde son rôle vide.

`GET /api/v1/folders` renvoie les dossiers du compte identifié par le cookie, dans le tableau `folders`. Chaque entrée contient `id`, `path`, `delimiter`, `hidden` et `role`. Les rôles possibles sont `inbox`, `sent`, `drafts`, `junk`, `trash` et `archive`. Un dossier personnel a le rôle `null`.

Le synchroniseur importe jusqu’à 50 en-têtes par lot, en commençant par les UID les plus récents. Pendant l’import, une seconde connexion vérifie INBOX toutes les cinq secondes. Le serveur conserve un extrait de la partie textuelle et lit les dossiers en lecture seule.

Les listes de fils sont disponibles sur `GET /api/v1/views/reception`, `/api/v1/views/flux`, `/api/v1/views/justificatifs` et `GET /api/v1/folders/{id}/messages`. Le paramètre `taille` accepte de 1 à 100 fils. Sans lui, la page suit le réglage `WEBMAIL_PAGE_SIZE`. Pour continuer, transmettez `curseur_suivant` dans le paramètre `curseur` de la même liste. Une fin de liste renvoie `null`. La date de réception et l’identifiant stable du fil déterminent l’ordre entre les pages.

`nouveaux` contient les fils avec au moins une lettre admise non lue. `deja_vus` contient les autres fils. Chaque résumé donne l’expéditeur, l’objet et l’extrait de sa dernière lettre visible, puis les indicateurs et le nombre de lettres du fil. `compteurs` compte les fils de chaque destination. `expediteurs_en_attente` compte les adresses distinctes à trier. La réponse fournit la révision de cette photographie SQLite. Les lectures HTTP utilisent l’index local pendant que le synchroniseur traite les prochains lots IMAP.

`couverture` décrit l’import du dossier consulté, ou d’INBOX pour les trois vues. Avant le décompte IMAP, `attendus` et `proportion_indexee` valent `null`. Après ce décompte, `indexes` et `attendus` comptent des lettres et `proportion_indexee` va de 0 à 1. Un dossier vide déjà vérifié a une couverture de 1.

### Parcourir la boîte dans le navigateur

La recette manuelle compare l’import des en-têtes, la lecture des corps et le nombre de non-lus sur une boîte distante et sur le banc local. [Résultats et commande de lecture seule](recette/infomaniak-lecture.md)

La session ouvre Réception. Le menu permet de rejoindre Flux, Justificatifs ou un dossier du compte. La touche H ouvre le menu et Échap rend le focus à son bouton. Le champ de recherche du menu reconnaît les noms accentués. Chaque vue garde ses pages chargées en mémoire pendant la session, avec le fil au focus, la section et l’ancre de défilement. Au retour, SolidJS affiche ces pages pendant qu’une requête actualise leur contenu. Les URL peuvent être rechargées directement, y compris `/dossier/{id}`.

La liste demande la page suivante quand il reste moins de 600 pixels sous la zone visible. « Afficher les fils suivants » fait la même demande au clavier et sert à réessayer après une coupure. Les pages s’ajoutent par identifiant de fil, dans l’ordre du serveur, et chaque vue les garde en mémoire avec sa section, son ancre et sa ligne au focus. Une actualisation relit toutes les pages chargées. La première requête suit la taille réglée et les suivantes demandent les lignes restantes, par tranches de 100 au plus.

Quand la liste change sous un lecteur qui a défilé, la ligne sous le pointeur garde sa place, ou à défaut la première ligne visible. Un fil plus récent que le dernier courrier vu en haut de liste fait apparaître « Nouveautés » avec leur nombre. Cette commande remonte en haut et place le focus sur la première ligne. En haut de liste, le courrier arrivé s’affiche directement.

Dans la liste, un clic sur l’objet ou la touche Entrée ouvre le fil. L’avatar est une case à cocher, utilisable avec Espace. « Sélectionner la page » agit sur les fils affichés dans les deux sections. La coche, le contour et le compteur indiquent la sélection. Chaque vue conserve ses propres choix. « Sélectionner la page » couvre toutes les pages chargées, et le navigateur répartit un lot de plus de 100 fils en plusieurs opérations. L’étoile agit sur sa ligne.

L’étoile, « Marquer comme lu » et « Marquer comme non lu » changent la liste dès le geste. `web/src/sync/operations.ts` garde l’intention dans une file, l’envoie sous un identifiant qu’il choisit, puis interroge le serveur jusqu’à sa conclusion. Une réponse perdue fait repartir la même demande avec le même identifiant, jusqu’à trois envois. Tant que la messagerie n’a pas confirmé, la ligne porte « En attente » et l’étoile un contour pointillé. Ces deux signes n’apparaissent qu’après 0,4 seconde. Un lot marqué quitte la sélection aussitôt. Si la messagerie refuse, les fils concernés retrouvent leur état, reviennent dans la sélection et « Modification non confirmée » donne leur nombre. Une fois l’opération conclue, la vue de l’action et les autres vues chargées qui montrent une cible sont relues.

`POST /api/v1/operations` enregistre une action et répond sans attendre la messagerie. Le corps contient `id`, `type`, `cibles` et, pour désigner des fils, `vue`. `id` est un UUID choisi par le client. `type` vaut `etoile`, `sans_etoile`, `lu`, `non_lu`, `archiver`, `corbeille`, `restaurer`, `supprimer_definitivement` ou `vider_dossier`. `cibles` compte de 1 à 100 identifiants. Avec `vue`, ils désignent les fils de cette vue et l’action couvre leurs messages visibles dans cette vue. Sans ce champ, fournissez des identifiants de messages obtenus avec `GET /api/v1/threads/{id}`. Par exemple, `{"id":"0190a1b2-0000-7000-8000-00000000000a","type":"non_lu","cibles":["1-1-3","1-1-4"]}` marque ces deux messages non lus.

La projection SQLite, l’opération et la nouvelle révision du compte sont écrites dans une seule transaction. La réponse donne `id`, `type`, `etat`, `revision` et un couple `id` et `etat` par cible dans `resultats`, dans l’ordre de la demande. L’opération est `en_attente`, puis `confirmee` si au moins un message est confirmé, sinon `refusee`. Une cible est `en_attente`, `confirme`, `partiel`, `refuse` ou `absent`. Une cible inconnue ou d’une autre boîte est `absent`, et une opération sans aucun message à modifier est `refusee` d’emblée. Renvoyer un `id` déjà connu du compte rend son résultat sans rien réappliquer. `GET /api/v1/operations/{id}` rend le même objet à tout moment.

Le synchroniseur applique les opérations d’un compte dans leur ordre d’arrivée, avec une commande `UID STORE` par dossier et par lot de 100 messages. Lu puis non lu aboutit donc à non lu, dans Pli et dans la boîte. Un refus du serveur IMAP, un dossier d’une autre génération ou un message disparu rétablit l’état d’avant pour les seuls messages concernés. Si une action plus récente attend sur le même message et le même indicateur, elle garde la main. Quand la messagerie est injoignable, l’opération reste `en_attente` et repart à la prochaine action, après 10 secondes ou au redémarrage. L’ouverture d’un message applique d’abord les opérations en attente du compte.

« Archiver la sélection » apparaît dans la barre de sélection, « Archiver le fil » ou « Archiver le message » sous le fil ouvert. Le libellé annonce la portée de l’action. Un fil d’un seul message s’archive comme un message. Les fils quittent la liste dès le geste, et le décompte de la page s’ajuste aussitôt. « Archivage en attente pour 2 fils. » reste dans la barre jusqu’à la conclusion, après le même délai de 0,4 seconde que les autres signes d’attente. Un fil refusé revient dans la liste, coché, et « Modification non confirmée » donne le nombre de refus. Le focus rejoint la case « Sélectionner la page », ou le titre de la liste quand elle s’est vidée. Après l’archivage du fil ouvert, la liste reprend sur la ligne voisine. Le dossier qui reçoit les archives ne propose pas la commande.

`Courrier` accepte `lectureEnSerie`. Quand il est vrai, l’archivage du fil ouvert affiche le fil suivant de la liste. Aucun réglage ne l’active encore.

L’opération `archiver` suit les mêmes règles que les autres. Avec `vue`, elle déplace les messages des fils visibles dans cette vue. Sans ce champ, elle déplace les messages désignés. Un message déjà rangé dans le dossier d’archives est `absent`. Sans dossier d’archives, `POST /api/v1/operations` répond 409 avec le code `archives_inconnues` et n’enregistre rien.

`GET /api/v1/folders` donne le rôle `archive` au seul dossier qui reçoit les archives et indique dans `selectable` si un dossier peut contenir du courrier. `PUT /api/v1/folders/archive` avec `{"dossier": 7}` retient pour le compte le dossier personnel qui les recevra, puis rend la liste des dossiers à jour. Ce choix l’emporte sur le rôle annoncé par la messagerie. Un dossier d’un autre compte, non ouvrable ou porteur d’un autre rôle est refusé avec 422 et le code `dossier_invalide`. Le navigateur ouvre cette demande au premier archivage d’un compte sans dossier d’archives, puis lance l’action suspendue.

Le synchroniseur cherche d’abord les messages encore présents avec `UID SEARCH`. Il les déplace avec `UID MOVE`. Sans cette extension, il enchaîne `UID COPY`, `UID STORE +FLAGS.SILENT (\Deleted)` et `UID EXPUNGE` sur ces seuls messages. Sans `MOVE` ni `UIDPLUS`, il refuse l’opération pour ne pas effacer d’autres messages marqués supprimés. Une seconde recherche tranche message par message. Un message parti est confirmé et rattaché à l’UID annoncé par `COPYUID`, un message resté revient dans ses listes. Sans UID annoncé, le message réapparaît à l’import suivant de son dossier d’arrivée. Un message déjà disparu est `absent`. Si la connexion tombe après l’envoi de la commande, le message parti entre-temps compte comme déplacé à la reprise. Un dossier d’arrivée supprimé depuis l’enregistrement refuse l’opération.

« Supprimer la sélection », « Supprimer le fil » et « Supprimer le message » déplacent les fils vers la Corbeille sans confirmation, comme un archivage. Dans la Corbeille, la barre et le fil ouvert proposent à la place « Restaurer » et « Supprimer définitivement ». « Vider le dossier » apparaît dans la Corbeille et les Indésirables tant qu’ils contiennent un fil. Une action définitive ouvre une boîte de dialogue qui cite la portée, par exemple « 2 fils seront supprimés définitivement de Corbeille. ». Le focus s’y ouvre sur « Annuler », Échap renonce et la fermeture rend le focus à la commande. Quand un fil de la Corbeille n’a pas de dossier d’origine connu, « Restaurer » demande d’abord où le remettre.

L’opération `corbeille` déplace les messages vers le premier dossier de rôle Corbeille et retient leur dossier d’origine. Sans ce dossier, `POST /api/v1/operations` répond 409 avec le code `corbeille_inconnue`. `restaurer` part de la Corbeille ou des Indésirables et ramène chaque message dans son dossier d’origine, tant qu’il existe. Pour les messages sans origine, la demande porte `destination`, l’identifiant d’un dossier ouvrable du compte. Sans lui, le serveur répond 409 avec le code `origine_inconnue` et n’enregistre rien. `supprimer_definitivement` ne vise que des messages de la Corbeille. `vider_dossier` reçoit dans `cibles` la vue d’un dossier de rôle Corbeille ou Indésirables, par exemple `["dossier/5"]`, et vise tous les messages que Pli y connaît à cet instant. Ces deux types effacent avec `UID STORE +FLAGS.SILENT (\Deleted)` puis `UID EXPUNGE` sur les seuls UID visés. Sans `UIDPLUS`, ils sont refusés.

Une page de liste donne dans `total` le nombre de fils de la liste entière. Dans la Corbeille et les Indésirables, un fil porte `sans_origine` quand un de ses messages n’a pas de dossier où revenir. Un message restauré dans la boîte de réception s’y lit de nouveau sans repasser par le tri, sauf si son expéditeur est refusé.

Le scénario de navigation dans `scripts/acceptation.sh` ouvre Chromium avec deux boîtes GreenMail fictives. Le banc utilise le navigateur de Playwright ou un Chromium système et installe celui de Playwright si nécessaire. `PLI_CHROMIUM_BIN` permet de choisir un exécutable. Pour conserver les captures de Réception, du menu et de la boîte vide aux trois formats, définissez `PLI_CAPTURES_NAVIGATION` avec le chemin absolu du dossier souhaité. Ajoutez `PLI_MESURE_RETOUR=1` pour chronométrer 100 retours sur ordinateur après 10 tours d’échauffement. Le fichier `mesure-retour.json` accompagne alors les captures.

Le scénario de pagination reçoit `WEBMAIL_PAGE_SIZE=4` et une boîte de vingt fils, dont dix-neuf à la même date. Il charge les pages au clavier puis par défilement, revient d’un dossier sur une ligne de la troisième page et accepte un expéditeur pendant ce temps. `PLI_CAPTURE_PAGINATION` reçoit le chemin d’une capture prise quand « Nouveautés » apparaît.

Le scénario des actions retient les réponses du serveur pendant qu’il étoile un fil, le marque lu puis non lu. Il vérifie ensuite l’état de la lettre dans la boîte, rejoue une opération, tente d’atteindre la lettre depuis une autre boîte et retire une lettre de la boîte pour provoquer un refus. `PLI_CAPTURE_OPERATIONS` reçoit le chemin d’une capture prise pendant que les réponses sont retenues, avec les signes d’attente.

Le scénario d’archivage passe par un relais IMAP du banc, lancé par `bench/screener.py --archivage`. Ce relais retire la dernière lettre du premier `UID MOVE` de plusieurs lettres : la messagerie confirme alors un fil et laisse l’autre sur place. La sonde consulte un fil, archive les deux autres pendant que la réponse est retenue, vérifie la boîte par IMAP, rejoue l’opération puis reprend le fil refusé. `PLI_CAPTURE_ARCHIVAGE` reçoit le chemin d’une capture prise pendant l’attente. Une seconde capture, suffixée `-avant`, montre la sélection. `PLI_TRACE_ARCHIVAGE` reçoit le chemin d’un fichier `.jsonl` où la sonde ajoute la trace de la reprise au format de `bench/README.md`. Le réseau est alors bridé comme pour la mesure du retour.

Le scénario de la corbeille supprime un fil pendant que la réponse est retenue, le restaure depuis la Corbeille, puis jette deux autres fils. Il annule un vidage, supprime définitivement un seul fil et vide le reste, en lisant la boîte par IMAP à chaque étape. `PLI_CAPTURE_CORBEILLE` reçoit le chemin d’une capture de la demande de confirmation du vidage.

### Vérifier le parcours de lecture de bout en bout

```sh
make e2e
```

La commande démarre un banc isolé, un serveur Pli et Chromium sans écran. Le scénario consulte une Réception en cours d’import, vérifie le tri à l’arrivée de deux messages, ouvre un fil et retrouve la décision après redémarrage. Il contrôle aussi le retour à la ligne de lecture à 390 × 844 pixels. Les services de test et la base temporaire sont arrêtés ou supprimés à la sortie.

Sur ordinateur, 10 retours d’échauffement précèdent 100 mesures du clic à la première ligne visible. Chromium simule 10 Mbit/s en réception, 2 Mbit/s en envoi et 80 ms de latence. Le contrôle échoue si le p95 atteint 50 ms ou si une requête précède l’affichage de la liste en cache. Les observations individuelles et la synthèse se trouvent dans `bench/traces/e2e/`. La CI joint ces fichiers à l’exécution du job `e2e`.

### Mesurer une page au milieu de 100 000 messages

```sh
cargo build --manifest-path server/Cargo.toml --locked --release
python3 bench/pages.py server/target/release/webmail-server
```

Le banc requiert Python 3 avec SQLite. Il démarre le serveur sur un port de bouclage et crée 100 000 lettres fictives réparties en 50 000 fils. Il suit les curseurs HTTP jusqu’au milieu de la boîte, chauffe la page de 50 fils puis chronomètre 100 requêtes sur une connexion persistante, jusqu’au décodage du JSON. Il vérifie les identifiants reçus et échoue à partir de 5 ms au p95. Sur le poste de développement le 27 novembre 2026, la médiane était de 0,295 ms et le p95 de 0,584 ms pour 14 207 octets par réponse.

`GET /api/v1/screener` renvoie le tableau `senders` pour la session ouverte. Chaque entrée regroupe les nouvelles lettres d’une adresse en attente et contient `address`, `name`, `last_subject`, `snippet`, `message_count` et `revision`. Les lettres d’INBOX antérieures au repère restent en Réception, même si la même adresse écrit à nouveau et attend une décision.

Depuis « Nouveaux expéditeurs », un choix explicite s’applique aux lettres de cette adresse déjà dans INBOX et à celles qui y arriveront. Accepter admet les fils en Réception, dans Flux ou dans Justificatifs. Refuser les retire des vues de lecture. « Historique du tri » dans le menu de l’avatar permet de consulter les lettres encore présentes et de réviser chaque choix. Le dossier Indésirables et les indicateurs IMAP conservent leur état.

Le catalogue détaillé est disponible sur `GET /api/v1/screener?details=true` et `GET /api/v1/screener/historique?details=true`. La page suivante utilise `apres` pour les adresses et `GET /api/v1/screener/messages?adresse=…&avant=…&generation=…` pour les lettres. `POST /api/v1/screener/decisions` reçoit `adresse`, `decision`, `destination` et `revision_attendue`. La réponse contient la décision enregistrée, sa révision et celle du compte. Un conflit HTTP 409 fournit ces mêmes champs pour que le second onglet puisse afficher le choix courant et le modifier explicitement.

Le cookie est propre à l’hôte, `Secure`, `HttpOnly` et `SameSite=Strict`. Servez Pli en HTTPS en déploiement et utilisez l’adresse de bouclage en développement. SQLite conserve l’empreinte SHA-256 du jeton et le secret IMAP chiffré en AES-256-GCM. Le chiffrement associe le secret à l’adresse et au profil serveur. Sauvegardez la clé séparément de la base pour pouvoir relire les secrets après restauration.

**Isolation des boîtes.** Chaque route de données de courrier reçoit l’extracteur Axum `CompteAuthentifie`. Ses lectures et écritures filtrent sur `account_id` issu de cette session, y compris lors d’un accès par identifiant de ressource. L’API de santé est publique. L’administration utilise son propre jeton.

## Structure

- `server/` produit le binaire unique `webmail-server`, point d’entrée de l’API et du synchroniseur. SQLite fonctionne en WAL avec une connexion d’écriture et quatre connexions réservées aux lectures. Chaque connexion active les clés étrangères.
- `server/migrations/` contient les migrations SQL numérotées. Ajoutez le prochain dossier, par exemple `002-nom/up.sql`, puis recompilez. Au démarrage, le binaire applique les migrations embarquées dans l’ordre et enregistre la version dans `PRAGMA user_version`. Un schéma plus récent que le binaire interrompt le démarrage.
- `server/src/http.rs` expose l’API sous `/api/v1`. Les erreurs JSON contiennent `code` et `message`. Toutes les réponses d’API portent `Cache-Control: no-store`. Axum sert aussi `web/dist/`. Le navigateur revalide `index.html` à chaque visite et peut conserver un an les fichiers à empreinte de `/assets/`.
- `web/src/` contient l’interface SolidJS et ses CSS Modules. **Aucun composant n’appelle directement `fetch`, IndexedDB ou EventSource.** Les composants passent par les actions métier exportées de `web/src/sync/index.ts`. Le client HTTP typé se trouve dans `web/src/sync/api.ts`.

Les versions exactes des dépendances sont fixées dans les fichiers de verrouillage du projet. [Serveur](../server/Cargo.lock) · [Interface](../web/package-lock.json)
