#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

if [[ $# -gt 1 || (${1-} != "" && ${1-} != mutation) ]]; then
    echo "Usage : $0 [mutation]" >&2
    exit 2
fi
if [[ -z ${PLI_APS_IMAPS_PORT:-} && -z ${PLI_APS_SMTPS_PORT:-} ]]; then
    exec bash bench/avec-greenmail.sh bash scripts/acceptation.sh "$@"
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
export WEBMAIL_SECRET_KEY
WEBMAIL_SECRET_KEY=$(openssl rand -hex 32)
# Seul le scénario d’expiration requiert un cookie très bref.
duree_session() {
    if [[ $1 == session-courrier ]]; then
        echo 3
    else
        echo 120
    fi
}
mkdir -p build/acceptance/{ir,dry}

for feature in "${features[@]}"; do
    nom=$(basename "$feature" .feature)
    ir="build/acceptance/ir/$nom.json"
    suite="acceptance/generated/$nom"
    aps gherkin-parser "$feature" "$ir"
    aps gherkin-ir-dry-checker "$ir" "build/acceptance/dry/$nom.json"
    acceptance-entrypoint-generator "$ir" "$suite"
    WEBMAIL_SESSION_TTL_SECONDS=$(duree_session "$nom") "$suite/run"
done

if [[ ${1-} == mutation ]]; then
    for feature in "${features[@]}"; do
        nom=$(basename "$feature" .feature)
        travail="build/acceptance-mutation/$nom"
        # APS annote la feature avec son manifeste. Il travaille sur une copie.
        mkdir -p "$travail/features"
        cp "$feature" "$travail/$feature"
        echo "Rapport de mutation : $travail/rapport.json"
        (cd "$travail" && WEBMAIL_SESSION_TTL_SECONDS=$(duree_session "$nom") aps gherkin-mutator --feature "$feature" --level full \
            --generated-dir "$racine/acceptance/generated/$nom" --work-dir . \
            --runner-worker "acceptance-runner --worker" --json) > "$travail/rapport.json"
    done
fi
