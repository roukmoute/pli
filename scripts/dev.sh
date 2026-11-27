#!/usr/bin/env bash
set -eu

target_dir=${CARGO_TARGET_DIR:-server/target}
cargo build --manifest-path server/Cargo.toml --locked --target-dir "$target_dir"

"$target_dir/debug/webmail-server" &
server_pid=$!
(cd web && exec ./node_modules/.bin/vite) &
vite_pid=$!

cleanup() {
    kill "$server_pid" "$vite_pid" 2>/dev/null || true
    wait "$server_pid" "$vite_pid" 2>/dev/null || true
}

trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
wait -n "$server_pid" "$vite_pid"
