"""Observe ou retire les lettres d’une boîte GreenMail de recette d’après leur objet."""

import imaplib
import json
import sys

from screener import HOST, IMAPS, tls_client
from seed import require_ok


def lettres(boite, objet):
    require_ok(boite.select("INBOX"))
    trouvees = require_ok(boite.uid("SEARCH", None, "SUBJECT", f'"{objet}"'))[0].split()
    if not trouvees:
        raise SystemExit(f"Aucune lettre « {objet} » dans la boîte.")
    return b",".join(trouvees)


def etat(boite, objet):
    drapeaux = require_ok(boite.uid("FETCH", lettres(boite, objet), "(FLAGS)"))
    lignes = [ligne.upper() for ligne in drapeaux if ligne]
    return {
        "non_lus": sum(b"\\SEEN" not in ligne for ligne in lignes),
        "etoiles": sum(b"\\FLAGGED" in ligne for ligne in lignes),
    }


def retirer(boite, objet):
    require_ok(boite.uid("STORE", lettres(boite, objet), "+FLAGS", "(\\Deleted)"))
    require_ok(boite.expunge())
    return {}


if __name__ == "__main__":
    commande, adresse, objet = sys.argv[1:]
    with imaplib.IMAP4_SSL(HOST, IMAPS, ssl_context=tls_client(), timeout=8) as boite:
        boite.login(adresse, adresse)
        print(json.dumps({"etat": etat, "retirer": retirer}[commande](boite, objet)))
