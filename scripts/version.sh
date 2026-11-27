#!/usr/bin/env bash
set -euo pipefail

fail() {
    printf 'Erreur : %s\n' "$1" >&2
    exit 1
}

dry_run=false
if [[ $# == 1 && $1 == --dry-run ]]; then
    dry_run=true
elif (( $# != 0 )); then
    fail "Usage : scripts/version.sh [--dry-run]"
fi

version_pattern='^v([0-9]+)\.([0-9]+)\.[0-9]+$'
for attempt in {1..5}; do
    [[ $(git branch --show-current) == main ]] || fail "La branche courante doit être main."
    changes=$(git status --porcelain=v1 --untracked-files=all --ignore-submodules=none)
    [[ -z $changes ]] || fail "L’arbre de travail doit être propre, y compris les fichiers non suivis."

    commit=$(git rev-parse HEAD)
    if [[ $dry_run == true ]]; then
        # Consulter directement main permet de simuler sans importer de tags.
        remote_main=$(git ls-remote --exit-code origin refs/heads/main)
        upstream=${remote_main%%$'\t'*}
    else
        git fetch origin --tags >&2
        upstream=$(git rev-parse --verify refs/remotes/origin/main)
    fi
    [[ $commit == "$upstream" ]] || fail "HEAD et origin/main doivent désigner le même commit."

    head_tags=$(git tag --list --points-at "$commit" --sort=-version:refname 'v*')
    while IFS= read -r tag; do
        if [[ $tag =~ $version_pattern ]]; then
            printf '%s\n' "$tag"
            exit 0
        fi
    done <<< "$head_tags"

    remote_tags=$(git ls-remote --tags --sort=-version:refname origin 'refs/tags/v*')
    version=
    while IFS=$'\t' read -r object ref; do
        # Pour un tag annoté, Git annonce aussi son commit sous refs/tags/<tag>^{}.
        tag=${ref#refs/tags/}
        tag=${tag%'^{}'}
        [[ $tag =~ $version_pattern ]] || continue
        if [[ $object == "$commit" ]]; then
            printf '%s\n' "$tag"
            exit 0
        fi
        if [[ -z $version ]]; then
            version="v${BASH_REMATCH[1]}.$((10#${BASH_REMATCH[2]} + 1)).0"
        fi
    done <<< "$remote_tags"
    version=${version:-v0.1.0}

    if [[ $dry_run == true ]]; then
        printf '%s\n' "$version"
        exit 0
    fi

    command -v gh >/dev/null || fail "La publication de la Release nécessite la commande gh."
    tag_ref="refs/tags/$version"
    tag_object=
    if git tag -a "$version" -m "$version" "$commit"; then
        tag_object=$(git rev-parse "$tag_ref")
        if git push --no-follow-tags origin "$tag_ref" >&2; then
            break
        fi
        # Retirer uniquement l’objet que cette tentative vient de créer.
        git update-ref -d "$tag_ref" "$tag_object"
    fi

    remote_tag=$(git ls-remote --refs origin "$tag_ref")
    [[ -n $remote_tag ]] || fail "La publication du tag $version a échoué."
    if [[ ${remote_tag%%$'\t'*} == "$tag_object" ]]; then
        # Le serveur a accepté notre tag, même si sa réponse au push a été perdue.
        git fetch origin "$tag_ref:$tag_ref" >&2
        break
    fi
    (( attempt < 5 )) || fail "Un autre agent a pris le numéro à chacune des cinq tentatives."
done

if ! gh release create "$version" --generate-notes --verify-tag >&2; then
    # Deux agents peuvent avoir poussé exactement le même objet de tag.
    if [[ $(gh release view "$version" --json isDraft --jq '.isDraft') != false ]]; then
        fail "Le tag est sur origin. Pour publier la Release, relancez : gh release create $version --generate-notes --verify-tag"
    fi
fi
printf '%s\n' "$version"
