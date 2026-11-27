#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

outils=$PWD/.outils
export PATH="$outils/bin:$PATH"
sources=(server web)

crap() {
    make web/node_modules/.installed
    mkdir -p target/coverage/rust
    cargo llvm-cov --manifest-path server/Cargo.toml --locked --workspace \
        --lcov --output-path target/coverage/rust/lcov.info
    npm --prefix web run coverage
    for rapport in target/coverage/{rust,typescript}/lcov.info; do
        if ! grep -q '^SF:' "$rapport"; then
            echo "Couverture absente de $rapport" >&2
            return 1
        fi
    done
    "$outils/venv/bin/crapper" --use-existing-coverage --threshold 30 "${sources[@]}"
}

muter() {
    make web/node_modules/.installed
    # Un worker réutilise sa compilation Rust. Chaque site sélectionné est
    # rejoué, même si seuls les tests ont changé ou si la ligne est non couverte.
    PLI_MUTATION_WEB="$PWD/web" "$outils/venv/bin/python" -u scripts/mutations.py "$@"
}

fichiers_modifies() {
    local tag
    if tag=$(git describe --tags --match 'v[0-9]*' --abbrev=0 2>/dev/null); then
        echo "Base de mutation : $tag ($(git rev-parse "$tag^{commit}"))" >&2
        git diff --name-only --no-renames --diff-filter=ACMT -z "$tag" -- "${sources[@]}" || return
    else
        echo "Base de mutation : aucun tag de version accessible, toutes les sources." >&2
        git ls-files --cached -z -- "${sources[@]}" || return
    fi
    git ls-files --others --exclude-standard -z -- "${sources[@]}"
}

muter_selection() {
    local chemin
    local fichiers=()
    while IFS= read -r -d '' chemin; do
        case "$chemin" in
            *.rs|*.ts|*.tsx|*.mts|*.cts|*.js|*.jsx|*.mjs|*.cjs)
                if [[ -f "$chemin" ]]; then
                    fichiers+=("$chemin")
                fi
                ;;
        esac
    done
    if (( ${#fichiers[@]} )); then
        muter "${fichiers[@]}"
    else
        echo "Aucun fichier source modifié depuis le dernier tag."
    fi
}

case "${1-}" in
    '')
        statut=0
        scripts/qualite.sh crap || statut=$?
        scripts/qualite.sh mutation || statut=$?
        scripts/qualite.sh dry
        exit "$statut"
        ;;
    crap)
        scripts/outils.sh qualite
        crap
        ;;
    mutation)
        shift
        scripts/outils.sh qualite
        if (( $# )); then
            for fichier in "$@"; do
                if [[ ! -f "$fichier" ]]; then
                    echo "Fichier introuvable : $fichier" >&2
                    exit 1
                fi
            done
            muter "$@"
        else
            fichiers_modifies | muter_selection
        fi
        ;;
    mutation-complete)
        scripts/outils.sh qualite
        muter "${sources[@]}"
        ;;
    dry)
        scripts/outils.sh qualite && "$outils/venv/bin/dryer" --threshold 0.82 "${sources[@]}" \
            || echo "Le rapport de duplication n’a pas pu être produit." >&2
        ;;
    *)
        echo "Usage : $0 [crap | mutation [fichiers…] | mutation-complete | dry]" >&2
        exit 1
        ;;
esac
