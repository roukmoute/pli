# Webmail Infomaniak

L’objectif est de proposer une interface inspirée de HEY pour lire, trier et envoyer les messages d’une boîte Infomaniak.

[Cahier des charges du webmail](Kanban/.specs/piece-TlW9Y4/cahier-des-charges-webmail-infomaniak-hey.md)

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

Le job `integration` du même workflow vérifie le jeu de 20 messages avec Docker. Pour démarrer le banc à la main, choisir un jeu ou consulter les capacités IMAP mesurées, suivez le [guide du banc GreenMail](bench/README.md).

## Écrire et lancer un test

`scripts/qualite.sh mutation` affiche le tag de départ et rejoue chaque site sélectionné. Les tests d’intégration utilisent un GreenMail jetable. Une équivalence documentée vaut pour une empreinte du fichier, une position et un remplacement précis. Si le fichier change, toute mutation encore survivante demande une nouvelle revue. Le code de sortie 2 signale un témoin en échec et le code 3 un survivant sans justification. [Équivalences examinées](scripts/mutations-equivalentes.toml)

**Rust.** Ajoutez un `#[test]` au module `tests` du fichier concerné. Pour tester une route, utilisez `#[tokio::test]`. L’instance créée par `let app = crate::test_support::TestApp::new()?;` conserve la base SQLite temporaire migrée, accessible par `app.db`, pendant toute sa portée. Importez `tower::ServiceExt` pour appeler le routeur en mémoire avec `app.router().oneshot(request).await?`. [Exemple de test HTTP](server/src/http.rs)

**Front.** Placez un fichier `*.test.tsx` à côté du composant ou `*.test.ts` à côté du module. Montez un composant avec `render` et attendez le texte affiché avec `findByText`. Pour le client HTTP, simulez `fetch` avec `vi.stubGlobal`. Vitest utilise jsdom, nettoie les composants montés et restaure les variables globales simulées entre les tests. [Test du composant](web/src/App.test.tsx) · [Test du client HTTP](web/src/sync/api.test.ts)

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

Sur `/admin`, ouvrez l’administration puis choisissez **Ajouter un serveur**. Le préréglage Infomaniak utilise `mail.infomaniak.com`, IMAP 993 en TLS implicite et SMTP 587 en STARTTLS. Le banc local propose IMAPS 3993 et SMTPS 3465. Les ports sont modifiables.

**Tester** affiche la résolution DNS, la connexion TCP, la négociation TLS et l’authentification pour chaque protocole. **Tester et enregistrer** applique le profil après la réussite des deux authentifications. Les identifiants servent uniquement à cette requête. Le diagnostic ne lit ni n’envoie de message.

L’API d’administration utilise l’en-tête `Authorization: Bearer <jeton>`. `GET /api/admin/v1/servers` renvoie les serveurs et les préréglages. `POST /api/admin/v1/servers/test` teste un candidat. `POST /api/admin/v1/servers` le crée et `PUT /api/admin/v1/servers/{id}` le modifie. Ces trois opérations reçoivent un objet JSON contenant `profile` (même structure qu’un préréglage) et `credentials` (champs `username` et `password`). Une sauvegarde refusée renvoie le diagnostic avec le statut HTTP 422 et conserve le profil en service.

Les scénarios de `scripts/acceptation.sh` démarrent un GreenMail Docker temporaire avec le jeu `base`. Pour réutiliser un banc déjà peuplé avec ce jeu, exportez ses deux ports dans `PLI_APS_IMAPS_PORT` et `PLI_APS_SMTPS_PORT`. Les scénarios utilisent les boîtes fictives d’Alice et de Bob du [banc GreenMail](bench/README.md).

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

Les listes de fils sont disponibles sur `GET /api/v1/views/reception`, `/api/v1/views/flux`, `/api/v1/views/justificatifs` et `GET /api/v1/folders/{id}/messages`. Le paramètre `taille` accepte de 1 à 100 fils et vaut 20 par défaut. Pour continuer, transmettez `curseur_suivant` dans le paramètre `curseur` de la même liste. Une fin de liste renvoie `null`. La date de réception et l’identifiant stable du fil déterminent l’ordre entre les pages.

`nouveaux` contient les fils avec au moins une lettre admise non lue. `deja_vus` contient les autres fils. Chaque résumé donne l’expéditeur, l’objet et l’extrait de sa dernière lettre visible, puis les indicateurs et le nombre de lettres du fil. `compteurs` compte les fils de chaque destination. `expediteurs_en_attente` compte les adresses distinctes à trier. La réponse fournit la révision de cette photographie SQLite. Les lectures HTTP utilisent l’index local pendant que le synchroniseur traite les prochains lots IMAP.

`couverture` décrit l’import du dossier consulté, ou d’INBOX pour les trois vues. Avant le décompte IMAP, `attendus` et `proportion_indexee` valent `null`. Après ce décompte, `indexes` et `attendus` comptent des lettres et `proportion_indexee` va de 0 à 1. Un dossier vide déjà vérifié a une couverture de 1.

### Parcourir la boîte dans le navigateur

La session ouvre Réception. Le menu permet de rejoindre Flux, Justificatifs ou un dossier du compte. La touche H ouvre le menu et Échap rend le focus à son bouton. Le champ de recherche du menu reconnaît les noms accentués. Chaque vue garde sa dernière page en mémoire pendant la session, avec le fil au focus, la section et l’ancre de défilement. Au retour, SolidJS affiche cette page pendant qu’une requête actualise son contenu. Les URL peuvent être rechargées directement, y compris `/dossier/{id}`.

Le scénario de navigation dans `scripts/acceptation.sh` ouvre Chromium avec deux boîtes GreenMail fictives. Le banc utilise le navigateur de Playwright ou un Chromium système et installe celui de Playwright si nécessaire. `PLI_CHROMIUM_BIN` permet de choisir un exécutable. Pour conserver les captures de Réception, du menu et de la boîte vide aux trois formats, définissez `PLI_CAPTURES_NAVIGATION` avec le chemin absolu du dossier souhaité. Ajoutez `PLI_MESURE_RETOUR=1` pour chronométrer 100 retours sur ordinateur après 10 tours d’échauffement. Le fichier `mesure-retour.json` accompagne alors les captures.

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

## Versions retenues

| Partie | Versions |
| --- | --- |
| Serveur | Rust 1.100.0, Axum 0.8.9, Tokio 1.54.1, tower-http 0.7.1, Serde 1.0.229 |
| Stockage | rusqlite 0.40.2 avec SQLite 3.53.2 embarqué (`bundled`), r2d2 0.8.10, r2d2_sqlite 0.35.0, rusqlite_migration 2.6.0 |
| Journalisation | tracing 0.1.44, tracing-subscriber 0.3.23 |
| Interface | Node 24.22.3, Vite 8.1.7, SolidJS et `@solidjs/web` 2.0.4, `@solidjs/vite-plugin` 3.0.1 |
| Vérification du front | Biome 2.5.15, TypeScript 7.0.2 |
| Tests du front | Vitest et `@vitest/coverage-v8` 5.1.2, jsdom 31.0.1, `@solidjs/testing-library` 1.0.0-rc.2 pour Solid 2 |
