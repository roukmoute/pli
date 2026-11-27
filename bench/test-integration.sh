#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

python3 -B -m unittest discover -s bench
exec bash bench/avec-greenmail.sh cargo test --manifest-path server/Cargo.toml \
    --locked --workspace --tests -- --ignored greenmail_
