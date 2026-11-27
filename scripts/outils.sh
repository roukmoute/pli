#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

outils=$PWD/.outils
mkdir -p "$outils"
paquets=()
while read -r nom sha; do
    depot="$outils/$nom"
    if [[ ! -d "$depot/.git" ]]; then
        gh auth status --active --hostname github.com >/dev/null
        gh repo clone "https://github.com/unclebob/$nom.git" "$depot"
    elif ! git -C "$depot" cat-file -e "$sha^{commit}" 2>/dev/null; then
        gh auth status --active --hostname github.com >/dev/null
        (cd "$depot" && gh repo sync --source "unclebob/$nom")
    fi
    if ! git -C "$depot" diff --quiet HEAD --; then
        echo "Le clone $depot contient des modifications locales." >&2
        exit 1
    fi
    if [[ $(git -C "$depot" rev-parse HEAD) != "$sha" ]] || git -C "$depot" symbolic-ref -q HEAD >/dev/null; then
        git -C "$depot" switch --detach "$sha"
    fi
    paquets+=(-e "$depot")
done < scripts/outils.lock

venv="$outils/venv"
if [[ ! -x "$venv/bin/python" ]]; then
    python3 -m venv "$venv"
fi
if ! cmp -s scripts/outils.lock "$venv/outils.lock"; then
    "$venv/bin/python" -m pip install "${paquets[@]}"
    cp scripts/outils.lock "$venv/outils.lock"
fi

version=0.9.2
if [[ $("$outils/bin/cargo-llvm-cov" llvm-cov --version 2>/dev/null) != "cargo-llvm-cov $version" ]]; then
    cargo install --locked --root "$outils" --version "=$version" cargo-llvm-cov
fi
