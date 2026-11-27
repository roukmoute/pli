# Consignes du dépôt Pli

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
