"""Mesure en boucle locale la page médiane de Réception, depuis une session HTTP réelle."""

import argparse
from contextlib import closing
from hashlib import sha256
from http.client import HTTPConnection
import json
import os
from pathlib import Path
import re
import secrets
import sqlite3
import statistics
import subprocess
import tempfile
import time


def peupler(dossier, jeton):
    with closing(sqlite3.connect(dossier / "webmail.db")) as db:
        db.execute("PRAGMA foreign_keys=ON")
        db.executescript("""
            BEGIN;
            INSERT INTO server_profiles VALUES(1,'Banc','127.0.0.1',993,'implicit',
                '127.0.0.1',465,'implicit','[]',1,1);
            INSERT INTO users VALUES(1);
            INSERT INTO accounts VALUES(1,1,'lecture@recette.test',1,x'');
            INSERT INTO folders(id,account_id,remote_path,role,uidvalidity,uidnext)
                VALUES(1,1,'INBOX','inbox',1,100001);
            INSERT INTO inbox_markers VALUES(1,1,100001);
            INSERT INTO folder_imports VALUES(1,1,100001,100000,0);
            WITH RECURSIVE serie(n) AS (VALUES(1) UNION ALL SELECT n+1 FROM serie WHERE n<100000)
            INSERT INTO messages(account_id,folder_id,uidvalidity,uid,sender_name,sender_address,
                recipients,subject,received_at,flags,size,has_attachments,snippet,thread_id)
            SELECT 1,1,1,n,'Atelier','atelier@recette.test','{}',printf('Courrier %d',n),
                1700000000+(n-1)/20,'[]',480,n%7=0,'Extrait de la lettre de recette.',
                printf('fil-%06d',(n+1)/2) FROM serie;
        """)
        db.execute("INSERT INTO sessions VALUES(?,1,unixepoch()+3600)", (sha256(jeton).digest(),))
        db.commit()


def mesurer(connexion, jeton):
    entetes = {"Cookie": f"__Host-pli_session={jeton.hex()}"}

    def page(curseur):
        chemin = "/api/v1/views/reception?taille=50"
        if curseur:
            chemin += f"&curseur={curseur}"
        connexion.request("GET", chemin, headers=entetes)
        reponse = connexion.getresponse()
        corps = reponse.read()
        if reponse.status != 200:
            raise RuntimeError(f"HTTP {reponse.status} sur la lecture")
        return json.loads(corps), len(corps)

    curseur = None
    for _ in range(500):
        donnees, _ = page(curseur)
        curseur = donnees["curseur_suivant"]
        if not curseur:
            raise RuntimeError("Fin de boîte avant le milieu attendu.")
    for _ in range(10):
        donnees, octets = page(curseur)
    attendus = [f"fil-{i:06d}" for i in range(25000, 24950, -1)]
    if (donnees["couverture"]["indexes"] != 100000 or donnees["compteurs"]["reception"] != 50000
            or [fil["id"] for fil in donnees["nouveaux"]] != attendus):
        raise RuntimeError("Le volume ou les 50 fils au milieu du jeu ne correspondent pas.")
    mesures = []
    for _ in range(100):
        debut = time.perf_counter_ns()
        donnees, octets = page(curseur)
        mesures.append((time.perf_counter_ns() - debut) / 1_000_000)
        if [fil["id"] for fil in donnees["nouveaux"]] != attendus:
            raise RuntimeError("Contenu inattendu durant la mesure.")
    p95 = sorted(mesures)[94]
    print(f"100 000 messages, 50 000 fils, 50 résumés par page, 100 requêtes, 0 erreur, "
          f"{octets} octets de JSON par réponse, médiane {statistics.median(mesures):.3f} ms, "
          f"p95 {p95:.3f} ms.")
    return p95


def main():
    options = argparse.ArgumentParser(description=__doc__)
    options.add_argument("binaire", type=Path)
    args = options.parse_args()
    with tempfile.TemporaryDirectory(prefix="pli-pages-") as temp:
        dossier = Path(temp)
        env = dict(os.environ, WEBMAIL_LISTEN="127.0.0.1:0", WEBMAIL_DATA_DIR=temp,
                   WEBMAIL_SECRET_KEY="", NO_COLOR="1")
        serveur = subprocess.Popen([args.binaire.resolve()], env=env, stdout=subprocess.PIPE, text=True)
        try:
            adresse = re.search(r"address=127\.0\.0\.1:(\d+)", serveur.stdout.readline())
            if not adresse:
                raise RuntimeError("Le service de recette n’a pas annoncé son port.")
            jeton = secrets.token_bytes(32)
            peupler(dossier, jeton)
            with closing(HTTPConnection("127.0.0.1", int(adresse[1]), timeout=5)) as connexion:
                return int(mesurer(connexion, jeton) >= 5)
        finally:
            serveur.terminate()
            serveur.wait()
            serveur.stdout.close()


if __name__ == "__main__":
    raise SystemExit(main())
