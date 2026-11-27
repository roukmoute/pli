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

# Un seul jeu de base pour la suite et ses mutations. Les ports publiés
# sont attribués par Docker, y compris si plusieurs suites tournent en parallèle.
if [[ -z ${PLI_APS_IMAPS_PORT:-} && -z ${PLI_APS_SMTPS_PORT:-} ]]; then
    export COMPOSE_PROJECT_NAME="pli-acceptation-$$"
    export PLI_BENCH_IMAP_PORT=0 PLI_BENCH_IMAPS_PORT=0 PLI_BENCH_SMTP_PORT=0 PLI_BENCH_SMTPS_PORT=0
    compose=(docker compose -f bench/docker-compose.yml)
    trap '"${compose[@]}" down' EXIT
    python3 bench/seed.py base
    adresse_imap=$("${compose[@]}" port greenmail 3993)
    adresse_smtp=$("${compose[@]}" port greenmail 3465)
    export PLI_APS_IMAPS_PORT=${adresse_imap##*:} PLI_APS_SMTPS_PORT=${adresse_smtp##*:}
fi

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
