"""Prépare une boîte GreenMail de recette dotée d’une corbeille, puis relève ses lettres."""

import imaplib
import json
import sys
from uuid import uuid4

from archivage import lettres
from screener import HOST, IMAPS, deliver, tls_client
from seed import require_ok


def preparer():
    adresse = f"corbeille-{uuid4().hex}@recette.test"
    deliver(adresse, ["Fil A", "Fil B", "Fil C", "Fil D"])
    with imaplib.IMAP4_SSL(HOST, IMAPS, ssl_context=tls_client(), timeout=8) as boite:
        boite.login(adresse, adresse)
        require_ok(boite.create("Corbeille"))
        require_ok(boite.subscribe("Corbeille"))
    return adresse


def observer(adresse):
    with imaplib.IMAP4_SSL(HOST, IMAPS, ssl_context=tls_client(), timeout=8) as boite:
        boite.login(adresse, adresse)
        return {
            "reception": [objet for objet, _ in lettres(boite, "INBOX")],
            "corbeille": [objet for objet, _ in lettres(boite, "Corbeille")],
        }


if __name__ == "__main__":
    if sys.argv[1:] == ["--preparer"]:
        print(preparer())
    else:
        print(json.dumps(observer(sys.argv[1])))
