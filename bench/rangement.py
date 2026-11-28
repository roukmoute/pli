"""Prépare deux boîtes GreenMail de recette avec leurs dossiers, puis relève ou retire un dossier."""

import imaplib
import json
import sys
from uuid import uuid4

from archivage import lettres
from screener import HOST, IMAPS, deliver, tls_client
from seed import require_ok

DOSSIERS = {"reception": "INBOX", "atelier": "Atelier", "chantier": "Chantier"}


def ouvrir(adresse):
    boite = imaplib.IMAP4_SSL(HOST, IMAPS, ssl_context=tls_client(), timeout=8)
    boite.login(adresse, adresse)
    return boite


def creer(adresse, *dossiers):
    with ouvrir(adresse) as boite:
        for dossier in dossiers:
            require_ok(boite.create(dossier))
            require_ok(boite.subscribe(dossier))


def preparer():
    adresse = f"rangement-{uuid4().hex}@recette.test"
    voisine = f"voisine-{uuid4().hex}@recette.test"
    deliver(adresse, ["Fil A", "Fil B", "Fil C", "Fil D", "Fil E"])
    deliver(voisine, ["Fil voisin"])
    creer(adresse, "Atelier", "Chantier")
    creer(voisine, "Voisinage")
    return f"{adresse} {voisine}"


def observer(adresse):
    with ouvrir(adresse) as boite:
        existants = b" ".join(require_ok(boite.list()))
        return {
            nom: [objet for objet, _ in lettres(boite, dossier)]
            for nom, dossier in DOSSIERS.items()
            if dossier.encode() in existants
        }


def retirer(adresse, dossier):
    with ouvrir(adresse) as boite:
        require_ok(boite.delete(dossier))


if __name__ == "__main__":
    if sys.argv[1] == "--preparer":
        print(preparer())
    elif sys.argv[1] == "--retirer":
        retirer(*sys.argv[2:])
    else:
        print(json.dumps(observer(sys.argv[1])))
