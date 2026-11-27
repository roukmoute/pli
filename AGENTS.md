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
