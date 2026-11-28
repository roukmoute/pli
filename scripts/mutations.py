"""Rejoue les sites avec le mutateur épinglé et vérifie chaque survivant."""

import hashlib
import os
from pathlib import Path
import sys
import tomllib

from mutator.cli import Options, select_files
from mutator.engine import mutate_file, restore_backups
from mutator.runner import CommandRunner


def commande_integration(chemin):
    if chemin.is_relative_to("server/tests"):
        return "cargo test --manifest-path server/Cargo.toml --locked -p webmail-server --tests -- --include-ignored"
    if chemin.as_posix() in {
        "server/acceptance/src/steps/archivage.rs",
        "server/acceptance/src/steps/decisions.rs",
        "server/acceptance/src/steps/folders.rs",
        "server/acceptance/src/steps/lecture.rs",
        "server/acceptance/src/steps/navigation.rs",
        "server/acceptance/src/steps/pages.rs",
        "server/acceptance/src/steps/screener.rs",
    }:
        return (
            "cargo test --manifest-path server/Cargo.toml --locked -p pli-acceptance --lib"
            " && bash scripts/acceptation.sh"
        )
    return None


def verifier(resultat, source, revue):
    # La position distingue deux opérateurs identiques sur une même ligne.
    # Une modification du fichier invalide sa revue, même à position égale.
    motifs = {}
    if hashlib.sha256(source).hexdigest() == revue.get("sha256"):
        motifs = revue["sites"]
    detectes = equivalents = inconnus = 0
    for site in resultat.sites:
        statut = resultat.statuses.get(site.mutation_id)
        colonne = site.start - source.rfind(b"\n", 0, site.start)
        position = f"{site.line}:{colonne} {site.description}"
        detail = f"{resultat.path}:{position}"
        if statut == "killed":
            detectes += 1
            print(f"KILLED    {detail}")
        elif statut == "survived" and position in motifs:
            equivalents += 1
            print(f"ÉQUIVALENT {detail} — {motifs[position]}")
        else:
            inconnus += 1
            print(f"{(statut or 'sans résultat').upper()} {detail}")
    return detectes, equivalents, inconnus


def main(chemins):
    racine = Path.cwd()
    options = Options(action="mutate", positionals=chemins)
    fichiers = select_files(options)
    commandes = {p: commande_integration(p.relative_to(racine)) for p in fichiers}
    ports = ("PLI_BENCH_IMAP_PORT", "PLI_BENCH_IMAPS_PORT", "PLI_BENCH_SMTPS_PORT",
             "PLI_APS_IMAPS_PORT", "PLI_APS_SMTPS_PORT")
    if any(commandes.values()) and not all(os.environ.get(p) for p in ports):
        os.execlp("bash", "bash", "bench/avec-greenmail.sh", sys.executable, "-u", __file__, *chemins)

    with Path(__file__).with_name("mutations-equivalentes.toml").open("rb") as fichier:
        revues = tomllib.load(fichier)
    for restaure in restore_backups(racine):
        print(f"Restauration après interruption : {restaure}", file=sys.stderr)
    runner = CommandRunner()
    temoins = {}
    detectes = equivalents = inconnus = 0
    for chemin, commande in commandes.items():
        print(f"Mutation de {chemin.relative_to(racine)}", flush=True)
        resultat = mutate_file(
            chemin, racine, runner=runner, covered_lines=None, ignore_coverage=True,
            mutate_all=True, lines=None, test_command=commande, max_workers=1,
            timeout_factor=options.timeout_factor, mutation_warning=options.mutation_warning,
            baselines=temoins,
        )
        if resultat.baseline_failed or resultat.skipped:
            print(resultat.baseline_message or resultat.skipped, file=sys.stderr)
            return 2
        tues, revus, autres = verifier(resultat, chemin.read_bytes(), revues.get(resultat.path, {}))
        detectes += tues
        equivalents += revus
        inconnus += autres
    print(f"{detectes + equivalents + inconnus} sites rejoués, {detectes} détectés, "
          f"{equivalents} équivalents, {inconnus} non justifiés.")
    return 3 if inconnus else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
