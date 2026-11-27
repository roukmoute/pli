# Consignes du dépôt Pli

## GitHub : toujours passer par gh

Avant toute interaction avec GitHub, vérifiez la connexion avec `gh auth status`. En cas d’échec, arrêtez-vous et signalez-le dans le journal de la carte. La commande `gh auth login` et tout changement de compte sont interdits.

Passez par `gh` pour toute interaction avec GitHub. Gérez les Releases avec `gh release` et les issues avec `gh issue`. Consultez le dépôt distant avec `gh repo view` et lancez les workflows manuels avec `gh workflow run`. Pour toute autre opération sur l’API GitHub, utilisez `gh api`. Les appels directs avec `curl` ou tout autre client HTTP sont interdits. N’écrivez jamais de jeton GitHub dans un fichier ou dans une variable versionnée.

Gardez `origin` en HTTPS (`https://github.com/roukmoute/pli.git`). Les remotes SSH sont interdits. Configurez Git avec `gh auth setup-git` pour que `git pull --rebase` et `git push` s’authentifient via `gh`.

Après chaque push, listez les exécutions de CI du commit poussé avec `gh run list --commit <sha>`. Suivez chacune avec `gh run watch <id> --exit-status`. En cas d’échec, corrigez la cause et vérifiez la CI du correctif avant de rendre la carte. Si le dépôt n’a aucun workflow ou si aucun ne se déclenche pour ce commit, notez-le dans le journal de la carte et poursuivez.

## Commits, push et versions

### Préparer les commits

Une carte produit généralement plusieurs petits commits. Donnez à chacun un seul changement cohérent. Après chaque commit, le projet doit compiler et ses tests doivent passer. Séparez les fonctionnalités, les refactorisations et la mise en forme.

Deux agents peuvent travailler en parallèle. Consultez `git status`, ajoutez chaque fichier de votre carte avec `git add -- <chemin>` et utilisez `git add -p` si un fichier contient aussi des modifications d’un autre agent. Relisez `git diff --staged` avant de committer. Évitez tout ajout global avec `git add -A`.

Suivez le format Conventional Commits `type(portée): résumé`. Écrivez le résumé à l’impératif et limitez la première ligne à 72 caractères. Les types admis sont `feat`, `fix`, `refactor`, `test`, `docs`, `build`, `ci`, `chore` et `perf`.

```text
feat(reception): affiche les messages non lus
fix(envoi): conserve le brouillon après un échec
test(tri): couvre le déplacement d’un fil
```

Le dépôt est public. Ne commitez jamais de secret, de fichier `.env*`, d’adresse de courrier réelle ni de contenu de courrier réel. Utilisez des données fictives dans les exemples et les tests.

### Pousser sur main

Travaillez directement sur `main`, sans branche de travail ni pull request. Avant chaque push de commits, synchronisez votre copie puis poussez sur GitHub.

```sh
git pull --rebase origin main
git push origin main
```

Si un autre agent a poussé entre-temps et que le push est refusé, répétez ces deux commandes. Ne réécrivez jamais un commit déjà poussé. Les options `--force` et `--force-with-lease` sont interdites.

### Publier une version par carte

Quand tous les critères de la carte sont remplis et tous ses commits poussés, lancez `scripts/version.sh` une seule fois, juste avant la remise en validation. Exécutez-le depuis un arbre propre sur `main`, au même commit que `origin/main`. Si le tableau Kanban est en cours de modification dans le dossier partagé, un clone propre de ce même commit convient.

Le script crée le tag annoté suivant (`v0.1.0`, puis `v0.2.0`, etc.), le pousse et publie la Release GitHub publique avec les notes générées par GitHub. Inscrivez la version affichée dans le journal de la carte.

## Règles de conception

### TypeScript

Composez les objets avec des fonctions et passez leurs dépendances en paramètres. Représentez les variantes d’un même type par une union discriminée. La seule forme d’héritage de classe autorisée est `extends Error` pour une classe d’erreur, comme `ApiError`. Depuis la racine, `npm --prefix web run lint` vérifie cette règle.

```ts
// À éviter
class Compteur extends DepotMessages {
  total() {
    return this.compterNonLus();
  }
}

// À faire
function creerCompteur(compterNonLus: () => number) {
  return { total: compterNonLus };
}
```

### Rust

Limitez chaque trait aux opérations dont son appelant a besoin. Passez les dépendances au constructeur, stockez-les dans des champs et déléguez-leur les opérations. N’utilisez pas `Deref` pour simuler l’héritage.

```rust
// À éviter
// Ce trait impose aussi l’envoi aux implémentations utilisées par le compteur.
trait Messagerie {
    fn compter_non_lus(&self) -> usize;
    fn envoyer(&self, texte: &str);
}

// À faire
trait LectureNonLus {
    fn compter_non_lus(&self) -> usize;
}

struct Compteur<L> {
    lecture: L,
}

impl<L: LectureNonLus> Compteur<L> {
    fn new(lecture: L) -> Self {
        Self { lecture }
    }

    fn total(&self) -> usize {
        self.lecture.compter_non_lus()
    }
}
```

## Qualité avant de rendre une carte

Depuis la racine, lancez `scripts/qualite.sh`. Au premier passage, le script
installe les outils de `scripts/outils.lock` dans `.outils/`. Prévoyez Python
3.11 ou ultérieur avec `venv`, ainsi que les prérequis habituels du projet et
une session `gh` connectée.

- `scripts/qualite.sh crap` mesure Rust et TypeScript. Un score CRAP supérieur
  à 30 bloque la remise de la carte. Testez la fonction ou simplifiez-la.
- `scripts/qualite.sh mutation` vérifie les sources modifiées depuis le dernier
  tag, y compris les nouveaux fichiers locaux. On peut aussi lui passer des
  chemins. Tuez chaque mutant survivant par un test, ou nommez-le dans le
  journal de la carte et justifiez sa survie (par exemple, un mutant équivalent).
  `scripts/qualite.sh mutation-complete` rejoue tous les sites de mutation
  pour le contrôle nocturne.
- `scripts/qualite.sh dry` signale les candidats à la duplication à partir de
  0,82 de similarité. Relisez chaque paire, puis factorisez-la ou expliquez
  dans le journal pourquoi vous gardez les deux fonctions. Ce rapport est
  consultatif.

## Tests d'acceptation (APS)

Chaque carte de fonctionnalité traduit ses critères d’acceptation en scénarios
Gherkin dans `features/`, avec un fichier par fonctionnalité. Écrivez les noms,
les étapes et les exemples en français métier. Le parseur APS reconnaît les
mots-clés anglais `Feature`, `Background`, `Scenario`, `Scenario Outline`,
`Examples`, `Given`, `When`, `Then` et `And`. Placez les valeurs attendues dans
la table d’exemples pour que la mutation puisse vérifier chacune d’elles.

Les étapes passent par l’API HTTP du service Rust. Chaque exemple démarre le
service sur un port local attribué par le système, avec sa propre base SQLite.
Quand le scénario utilise du courrier, appuyez-vous sur le banc IMAP/SMTP de test.

Avant de rendre une carte, lancez `scripts/acceptation.sh` depuis la racine.
Le script utilise Babashka ou, à défaut, compile les trois outils APS avec Go
1.22 ou ultérieur. Le premier clonage utilise la session `gh` du projet.
Les représentations JSON et les rapports de duplication se trouvent dans
`build/acceptance/`. Pour vérifier les scénarios ajoutés, lancez
`scripts/acceptation.sh mutation`. Retrouvez les valeurs mutées et le bilan de
chaque fonctionnalité dans `build/acceptance-mutation/`. L’adaptateur passe le
chemin du JSON muté dans `PLI_ACCEPTANCE_IR` au même point d’entrée
`acceptance/generated/<nom>/run` que celui de l’exécution normale.

Pour ajouter une étape, ajoutez son texte au scénario, puis implémentez-le dans
`World::step` de `server/acceptance/src/steps.rs`. Par exemple, pour
`Then l’état annoncé est <etat>`, le gestionnaire reconnaît le préfixe
`l’état annoncé est ` avec `parameter`, lit la colonne capturée avec
`example_value` et compare cette valeur au champ JSON `status` de la réponse
HTTP. Couvrez l’étape avec un exemple qui passe et un exemple qui échoue.
