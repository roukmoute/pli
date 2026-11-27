# Webmail Infomaniak

L’objectif est de proposer une interface inspirée de HEY pour lire, trier et envoyer les messages d’une boîte Infomaniak.

[Cahier des charges du webmail](Kanban/.specs/piece-TlW9Y4/cahier-des-charges-webmail-infomaniak-hey.md)

## Démarrer Pli

Depuis la racine du dépôt, avec rustup, un compilateur C, GNU Make et Node 24 muni de npm, lancez `make dev`. Si vous utilisez nvm, `nvm install` sélectionne la version du fichier `.nvmrc`. Rustup installe la chaîne du projet au premier appel à Cargo. Le lancement en développement utilise Bash 4.3 ou ultérieur.

Ouvrez <http://127.0.0.1:5173>. L’accueil vérifie la disponibilité du serveur et de sa base. Vite transmet les requêtes `/api` à Axum. Ctrl+C arrête les deux processus.

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
| `WEBMAIL_ADMIN_TOKEN` | vide | Jeton à saisir sur `/admin`. L’accès à l’administration exige un jeton configuré côté serveur. |
| `WEBMAIL_ALLOW_TEST_PROFILE` | `0` | La valeur `1` autorise le profil GreenMail local avec certificat de test sur une IP de bouclage. |

Sur `/admin`, ouvrez l’administration puis choisissez **Ajouter un serveur**. Le préréglage Infomaniak utilise `mail.infomaniak.com`, IMAP 993 en TLS implicite et SMTP 587 en STARTTLS. Le banc local propose IMAPS 3993 et SMTPS 3465. Les ports sont modifiables.

**Tester** affiche la résolution DNS, la connexion TCP, la négociation TLS et l’authentification pour chaque protocole. **Tester et enregistrer** applique le profil après la réussite des deux authentifications. Les identifiants servent uniquement à cette requête. Le diagnostic ne lit ni n’envoie de message.

L’API d’administration utilise l’en-tête `Authorization: Bearer <jeton>`. `GET /api/admin/v1/servers` renvoie les serveurs et les préréglages. `POST /api/admin/v1/servers/test` teste un candidat. `POST /api/admin/v1/servers` le crée et `PUT /api/admin/v1/servers/{id}` le modifie. Ces trois opérations reçoivent un objet JSON contenant `profile` (même structure qu’un préréglage) et `credentials` (champs `username` et `password`). Une sauvegarde refusée renvoie le diagnostic avec le statut HTTP 422 et conserve le profil en service.

Les scénarios de `scripts/acceptation.sh` démarrent un GreenMail Docker temporaire et attendent son démarrage avec OpenSSL. Pour réutiliser un banc local déjà lancé, exportez ses deux ports dans `PLI_APS_IMAPS_PORT` et `PLI_APS_SMTPS_PORT`. Le scénario utilise le compte fictif `diagnostic` avec le mot de passe de recette `essai-local`.

Make génère `.env.example` à partir de `config/example.env`. La commande `make .env.example` permet aussi de le préparer seul. Pour utiliser vos valeurs dans le terminal courant, copiez le modèle dans `.env`, modifiez-le, puis exportez ses variables avant de lancer le service.

```sh
set -a
. ./.env
set +a
```

## Structure

- `server/` produit le binaire unique `webmail-server`, point d’entrée de l’API et du futur synchroniseur. SQLite fonctionne en WAL avec une connexion d’écriture et quatre connexions réservées aux lectures. Chaque connexion active les clés étrangères.
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
