#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

if [[ $# -gt 1 || (${1-} != "" && ${1-} != mutation) ]]; then
    echo "Usage : $0 [mutation]" >&2
    exit 2
fi
shopt -s nullglob
features=(features/*.feature)
if (( ${#features[@]} == 0 )); then
    echo "Aucun fichier features/*.feature à exécuter." >&2
    exit 1
fi

racine=$PWD
scripts/outils.sh aps
if command -v bb >/dev/null; then
    aps() { bb --config "$racine/.outils/aps/bb.edn" "$@"; }
else
    aps() { local outil=$1; shift; "$racine/.outils/bin/$outil" "$@"; }
fi

cible=${CARGO_TARGET_DIR:-server/target}
cargo build --manifest-path server/Cargo.toml --locked -p pli-acceptance --bins --target-dir "$cible"
export PATH="$(cd "$cible/debug" && pwd):$PATH"
unset PLI_ACCEPTANCE_IR
mkdir -p build/acceptance/{ir,dry}

for feature in "${features[@]}"; do
    nom=$(basename "$feature" .feature)
    ir="build/acceptance/ir/$nom.json"
    suite="acceptance/generated/$nom"
    aps gherkin-parser "$feature" "$ir"
    aps gherkin-ir-dry-checker "$ir" "build/acceptance/dry/$nom.json"
    acceptance-entrypoint-generator "$ir" "$suite"
    "$suite/run"
done

if [[ ${1-} == mutation ]]; then
    for feature in "${features[@]}"; do
        nom=$(basename "$feature" .feature)
        travail="build/acceptance-mutation/$nom"
        # APS annote la feature avec son manifeste. Il travaille sur une copie.
        mkdir -p "$travail/features"
        cp "$feature" "$travail/$feature"
        echo "Rapport de mutation : $travail/rapport.json"
        (cd "$travail" && aps gherkin-mutator --feature "$feature" --level full \
            --generated-dir "$racine/acceptance/generated/$nom" --work-dir . \
            --runner-worker "acceptance-runner --worker" --json) > "$travail/rapport.json"
    done
fi
