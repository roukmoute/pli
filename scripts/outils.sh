#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

outils=$PWD/.outils
mkdir -p "$outils"
selection=${1:-tous}
case "$selection" in
    tous|qualite|aps) ;;
    *) echo "Usage : $0 [tous | qualite | aps]" >&2; exit 2 ;;
esac
paquets=()
while read -r nom sha; do
    projet=$nom
    if [[ $nom == aps ]]; then
        [[ $selection != qualite ]] || continue
        projet=Acceptance-Pipeline-Specification
    elif [[ $selection == aps ]]; then
        continue
    fi
    depot="$outils/$nom"
    if [[ ! -d "$depot/.git" ]]; then
        gh auth status --active --hostname github.com >/dev/null
        gh repo clone "https://github.com/unclebob/$projet.git" "$depot"
    elif ! git -C "$depot" cat-file -e "$sha^{commit}" 2>/dev/null; then
        gh auth status --active --hostname github.com >/dev/null
        (cd "$depot" && gh repo sync --source "unclebob/$projet")
    fi
    if ! git -C "$depot" diff --quiet HEAD --; then
        echo "Le clone $depot contient des modifications locales." >&2
        exit 1
    fi
    if [[ $(git -C "$depot" rev-parse HEAD) != "$sha" ]] || git -C "$depot" symbolic-ref -q HEAD >/dev/null; then
        git -C "$depot" switch --detach "$sha"
    fi
    if [[ $nom != aps ]]; then
        paquets+=(-e "$depot")
    fi
done < scripts/outils.lock

if [[ $selection != qualite ]] && ! command -v bb >/dev/null; then
    command -v go >/dev/null || { echo "APS requiert Babashka ou Go (1.22 minimum)." >&2; exit 1; }
    mkdir -p "$outils/bin"
    (cd "$outils/aps" && go build -o "$outils/bin/" ./cmd/...)
fi
[[ $selection != aps ]] || exit 0

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
