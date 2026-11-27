#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

# Chaque suite obtient son propre serveur, même si le banc manuel est ouvert.
export COMPOSE_PROJECT_NAME="pli-integration-$$"
export PLI_BENCH_IMAP_PORT=0 PLI_BENCH_IMAPS_PORT=0 PLI_BENCH_SMTP_PORT=0 PLI_BENCH_SMTPS_PORT=0
compose=(docker compose -f bench/docker-compose.yml)
trap '"${compose[@]}" down' EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

python3 -B -m unittest discover -s bench
python3 bench/seed.py base
address=$("${compose[@]}" port greenmail 3143)
imap_tls=$("${compose[@]}" port greenmail 3993)
smtp_tls=$("${compose[@]}" port greenmail 3465)
export PLI_BENCH_IMAP_PORT=${address##*:} PLI_BENCH_IMAPS_PORT=${imap_tls##*:} PLI_BENCH_SMTPS_PORT=${smtp_tls##*:}
cargo test --manifest-path server/Cargo.toml \
    --locked --workspace --tests -- --ignored greenmail_
