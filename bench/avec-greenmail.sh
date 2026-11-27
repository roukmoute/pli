#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

# Le serveur et ses ports appartiennent uniquement à la commande lancée ici.
export COMPOSE_PROJECT_NAME="pli-greenmail-$$"
export PLI_BENCH_IMAP_PORT=0 PLI_BENCH_IMAPS_PORT=0 PLI_BENCH_SMTP_PORT=0 PLI_BENCH_SMTPS_PORT=0
compose=(docker compose -f bench/docker-compose.yml)
trap '"${compose[@]}" down' EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
python3 bench/seed.py base
imap=$("${compose[@]}" port greenmail 3143)
imaps=$("${compose[@]}" port greenmail 3993)
smtps=$("${compose[@]}" port greenmail 3465)
export PLI_BENCH_IMAP_PORT=${imap##*:} PLI_BENCH_IMAPS_PORT=${imaps##*:} PLI_BENCH_SMTPS_PORT=${smtps##*:}
export PLI_APS_IMAPS_PORT=$PLI_BENCH_IMAPS_PORT PLI_APS_SMTPS_PORT=$PLI_BENCH_SMTPS_PORT
"$@"
