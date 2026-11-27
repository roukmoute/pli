"""Recette manuelle de lecture d’une boîte dédiée. Sortie limitée aux constats publiables."""

from contextlib import ExitStack, closing, contextmanager
from datetime import datetime, timezone
from functools import partial
from http.client import HTTPConnection, HTTPException
from pathlib import Path
from tempfile import TemporaryDirectory, TemporaryFile
from time import monotonic, sleep
from urllib.parse import quote
import imaplib
import json
import os
import re
import secrets
import ssl
import subprocess
import sys

from lecture_imap import Echec, capacites, compteurs, verifier_corps


RACINE = Path(__file__).resolve().parents[1]
HOTE = "mail.infomaniak.com"
ROLES = ("inbox", "sent", "drafts", "junk", "trash", "archive")
ETAPES = ("Configuration", "Authentification IMAP", "Capacités", "État initial",
          "Rattachement dans Pli", "Dossiers", "Repère de Réception", "Première page",
          "Import de Réception", "Texte", "HTML", "Pièces jointes", "Non-lus avant et après")


def identifiants(chemin):
    if not chemin.is_file():
        raise Echec("Le fichier local .env.recette est absent.")
    valeurs = {}
    for ligne in chemin.read_text().splitlines():
        cle, sep, valeur = ligne.strip().removeprefix("export ").partition("=")
        cle, valeur = cle.strip(), valeur.strip()
        if sep and cle in ("RECETTE_IMAP_USER", "RECETTE_IMAP_PASSWORD"):
            if len(valeur) >= 2 and valeur[0] == valeur[-1] and valeur[0] in "\"'":
                valeur = valeur[1:-1]
            valeurs[cle] = valeur
    adresse = valeurs.get("RECETTE_IMAP_USER", "")
    secret = valeurs.get("RECETTE_IMAP_PASSWORD", "")
    if not re.fullmatch(r"[^@\s]+@[^@\s]+", adresse) or not secret:
        raise Echec("Vérifier RECETTE_IMAP_USER et RECETTE_IMAP_PASSWORD dans .env.recette.")
    return adresse, secret


class Api:
    def __init__(self, port):
        self.port = port
        self.cookie = ""

    def requete(self, methode, chemin, corps=None, headers=None):
        with closing(HTTPConnection("127.0.0.1", self.port, timeout=180)) as connexion:
            connexion.request(methode, chemin, body=corps, headers=headers or {})
            reponse = connexion.getresponse()
            if reponse.status != 200:
                raise Echec(f"Le service local a répondu HTTP {reponse.status}.")
            return json.loads(reponse.read()), reponse.getheader("Set-Cookie")

    def connecter(self, adresse, secret):
        donnees = json.dumps({"address": adresse, "password": secret})
        _, cookie = self.requete("POST", "/api/v1/session", donnees, {"Content-Type": "application/json"})
        if not cookie:
            raise Echec("La session locale est absente.")
        self.cookie = cookie.partition(";")[0]

    def get(self, chemin):
        return self.requete("GET", chemin, headers={"Cookie": self.cookie})[0]


@contextmanager
def service(binaire, dossier, local=False):
    jeton = secrets.token_hex(32)
    env = dict(os.environ, WEBMAIL_LISTEN="127.0.0.1:0", WEBMAIL_DATA_DIR=str(dossier),
               WEBMAIL_SECRET_KEY=secrets.token_hex(32), WEBMAIL_ADMIN_TOKEN=jeton,
               WEBMAIL_ALLOW_TEST_PROFILE="1" if local else "0", NO_COLOR="1")
    with TemporaryFile(dir=dossier) as journal:
        processus = subprocess.Popen([binaire], cwd=RACINE, env=env, stdout=journal, stderr=journal)
        try:
            limite = monotonic() + 15
            while processus.poll() is None and monotonic() < limite:
                depart = os.pread(journal.fileno(), 4096, 0)
                port = re.search(rb"address=127\.0\.0\.1:(\d+)", depart)
                if port:
                    yield Api(int(port[1])), jeton
                    return
                sleep(.05)
            raise Echec("Le service local n’a pas annoncé son port.")
        finally:
            processus.terminate()
            try:
                processus.wait(timeout=5)
            except subprocess.TimeoutExpired:
                processus.kill()
                processus.wait()


def installer_profil(api, jeton, db, domaine, *, local=False, port_banc=None):
    catalogue, _ = api.requete("GET", "/api/admin/v1/servers", headers={"Authorization": f"Bearer {jeton}"})
    profil = next(p for p in catalogue["presets"] if p["local_test"] == local)
    imap, smtp = profil["imap"], profil["smtp"]
    if local:
        imap["port"] = port_banc
    elif imap != {"host": HOTE, "port": 993, "tls": "implicit"}:
        raise Echec("Le préréglage ne correspond pas au point d’accès IMAP attendu.")
    # Éviter le diagnostic SMTP de l’administration lors de cette recette de lecture.
    db.execute("""INSERT INTO server_profiles(name,imap_host,imap_port,imap_tls,smtp_host,
                  smtp_port,smtp_tls,domains,active,local_test) VALUES (?,?,?,?,?,?,?,?,1,?)""",
               (profil["name"], imap["host"], imap["port"], imap["tls"], smtp["host"], smtp["port"],
                smtp["tls"], json.dumps([domaine]), local))
    db.commit()


def premiere_page(api, debut, attendu):
    while monotonic() - debut < 600:
        page = api.get("/api/v1/views/reception?taille=20")
        fils = page["nouveaux"] + page["deja_vus"]
        if len(fils) > 20 or page["vue"] != "reception":
            raise Echec("La première page de Réception est incohérente.")
        if fils:
            entetes = api.get("/api/v1/threads/" + quote(fils[0]["id"], safe=""))
            if not entetes or any(not message["id"] for message in entetes):
                raise Echec("Les en-têtes du premier fil sont absents.")
            return
        if attendu == 0 and page["couverture"]["attendus"] == 0:
            return
        sleep(.2)
    raise Echec("La première page n’est pas devenue disponible en dix minutes.")


def attendre_import(api, debut, attendu):
    while monotonic() - debut < 600:
        couverture = api.get("/api/v1/views/reception?taille=1")["couverture"]
        if couverture["attendus"] is not None and not couverture["import_en_cours"]:
            if couverture["indexes"] != couverture["attendus"] or couverture["indexes"] != attendu:
                raise Echec("L’index et le témoin IMAP diffèrent. Vérifier les arrivées simultanées.")
            return
        sleep(.2)
    raise Echec("L’import est encore incomplet après dix minutes.")


def erreur_publique(erreur):
    if isinstance(erreur, Echec):
        return str(erreur)
    if isinstance(erreur, TimeoutError):
        return "Le délai de réponse est dépassé."
    if isinstance(erreur, ssl.SSLError):
        return "La vérification TLS a échoué."
    if isinstance(erreur, imaplib.IMAP4.error):
        return "Le serveur a refusé une commande IMAP."
    if isinstance(erreur, HTTPException):
        return "La réponse HTTP est illisible."
    return "Le contrôle n’a pas abouti."


class Recette:
    def __init__(self):
        self.resultats = {cle: ("Non exécuté", "") for cle in ETAPES}

    @contextmanager
    def etape(self, cle):
        try:
            yield
        except Exception as erreur:
            self.resultats[cle] = ("Échec", erreur_publique(erreur))
            raise
        if self.resultats[cle][0] == "Non exécuté":
            self.ok(cle)

    def ok(self, cle, detail="Vérifié."):
        self.resultats[cle] = ("OK", detail)

    def observer(self, boite, connecter, api, db, adresse, secret):
        with self.etape("Capacités"):
            noms = capacites(boite)
            self.ok("Capacités", ", ".join(f"{nom} {'oui' if oui else 'non'}" for nom, oui in noms.items()) + ".")
        with self.etape("État initial"):
            avant = compteurs(boite)
            inbox = avant[b"INBOX"]
            self.ok("État initial", "Compteurs relevés dans chaque dossier sélectionnable.")
        try:
            self.parcours(boite, api, db, adresse, secret, inbox)
        finally:
            with self.etape("Non-lus avant et après"):
                # Seconde session IMAP indépendante du dossier éventuellement sélectionné.
                with connecter() as temoin:
                    apres = compteurs(temoin)
                reference = {n: (e["UIDVALIDITY"], e["UNSEEN"]) for n, e in avant.items()}
                final = {n: (e["UIDVALIDITY"], e["UNSEEN"]) for n, e in apres.items()}
                if final != reference:
                    raise Echec("Les dossiers, leurs générations ou leurs non-lus ont changé pendant l’essai.")
                self.ok("Non-lus avant et après", "Écart de 0 dans chaque dossier sélectionnable, Réception comprise.")

    def parcours(self, boite, api, db, adresse, secret, inbox):
        debut = monotonic()
        with self.etape("Rattachement dans Pli"):
            api.connecter(adresse, secret)
        with self.etape("Dossiers"):
            vus = api.get("/api/v1/folders")["folders"]
            roles = {d["role"] for d in vus}
            if "inbox" not in roles:
                raise Echec("Le rôle de Réception n’a pas été reconnu.")
            self.ok("Dossiers", ", ".join(f"{r} {'oui' if r in roles else 'non'}" for r in ROLES) + ".")
        with self.etape("Repère de Réception"):
            repere = db.execute("SELECT uidvalidity, uidnext FROM inbox_markers").fetchone()
            if repere != (inbox["UIDVALIDITY"], inbox["UIDNEXT"]):
                raise Echec("Le repère de rattachement diffère du témoin IMAP.")
        with self.etape("Première page"):
            premiere_page(api, debut, inbox["MESSAGES"])
            self.ok("Première page", f"Page disponible {monotonic() - debut:.2f} s après le début du rattachement.")
        with self.etape("Import de Réception"):
            attendre_import(api, debut, inbox["MESSAGES"])
            self.ok("Import de Réception", f"En-têtes indexés en {monotonic() - debut:.2f} s depuis le rattachement.")
        try:
            verifier_corps(boite, api, db, self.resultats)
        except Exception as erreur:
            for cle in ("Texte", "HTML", "Pièces jointes"):
                if self.resultats[cle][0] == "Non exécuté":
                    self.resultats[cle] = ("Échec", erreur_publique(erreur))
            raise

    def afficher(self):
        print("## Relevé du", datetime.now(timezone.utc).strftime("%d/%m/%Y (UTC)"))
        print("\n| Étape | Résultat | Observation |\n| --- | --- | --- |")
        for cle, (etat, detail) in self.resultats.items():
            print(f"| {cle} | {etat} | {detail} |")
        return all(etat == "OK" for etat, _ in self.resultats.values())


@contextmanager
def ouvrir_imap(hote, port, tls, adresse, secret):
    with imaplib.IMAP4_SSL(hote, port, ssl_context=tls, timeout=30) as boite:
        boite.login(adresse, secret)
        yield boite


def main(binaire, banc=False):
    recette = Recette()
    try:
        with ExitStack() as ressources:
            with recette.etape("Configuration"):
                if os.environ.get("CI"):
                    raise Echec("Cette recette se lance uniquement hors CI.")
                import sqlite3
                if banc:
                    from screener import tls_client
                    hote, port, tls = "127.0.0.1", int(os.environ["PLI_BENCH_IMAPS_PORT"]), tls_client()
                    adresse, secret = "alice@recette.test", "alice-test"
                else:
                    hote, port, tls = HOTE, 993, ssl.create_default_context()
                    adresse, secret = identifiants(RACINE / ".env.recette")
                dossier = Path(ressources.enter_context(TemporaryDirectory(prefix="pli-recette-lecture-")))
                api, jeton = ressources.enter_context(service(binaire, dossier, local=banc))
                db = ressources.enter_context(closing(sqlite3.connect(dossier / "webmail.db")))
                installer_profil(api, jeton, db, adresse.rsplit("@", 1)[1], local=banc, port_banc=port)
            connecter = partial(ouvrir_imap, hote, port, tls, adresse, secret)
            with recette.etape("Authentification IMAP"):
                boite = ressources.enter_context(connecter())
            recette.observer(boite, connecter, api, db, adresse, secret)
    except Exception:
        # Une exception IMAP, SQLite ou HTTP peut contenir l’adresse, un dossier ou un objet.
        if all(etat != "Échec" for etat, _ in recette.resultats.values()):
            cle = next((c for c, (etat, _) in recette.resultats.items() if etat == "Non exécuté"), "Configuration")
            recette.resultats[cle] = ("Échec", "La vérification a été interrompue.")
    return 0 if recette.afficher() else 1


if __name__ == "__main__":
    import argparse
    options = argparse.ArgumentParser(description=__doc__)
    options.add_argument("binaire", type=Path)
    options.add_argument("--banc", action="store_true", help="comparer avec le banc local isolé")
    arguments = options.parse_args()
    raise SystemExit(main(arguments.binaire.resolve(), arguments.banc))
