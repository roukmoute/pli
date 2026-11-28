"""Relève les objets des lettres de Réception et d’Archives d’une boîte GreenMail de recette."""

import imaplib
import json
import sys
from email import policy
from email.parser import BytesParser

from screener import HOST, IMAPS, tls_client
from seed import require_ok


def lettres(boite, dossier):
    require_ok(boite.select(dossier, readonly=True))
    uids = require_ok(boite.uid("SEARCH", None, "ALL"))[0].split()
    if not uids:
        return []
    reponse = require_ok(
        boite.uid("FETCH", b",".join(uids), "(FLAGS BODY.PEEK[HEADER.FIELDS (SUBJECT)])")
    )
    trouvees = []
    for partie in reponse:
        if isinstance(partie, tuple):
            attributs, entete = partie
            objet = BytesParser(policy=policy.default).parsebytes(entete)["Subject"]
            trouvees.append((str(objet), b"\\SEEN" in attributs.upper()))
    return sorted(trouvees)


def observer(adresse):
    with imaplib.IMAP4_SSL(HOST, IMAPS, ssl_context=tls_client(), timeout=8) as boite:
        boite.login(adresse, adresse)
        reception = lettres(boite, "INBOX")
        archives = lettres(boite, "Archives")
    return {
        "reception": [objet for objet, _ in reception],
        "lues": [objet for objet, lue in reception if lue],
        "archives": [objet for objet, _ in archives],
    }


if __name__ == "__main__":
    print(json.dumps(observer(sys.argv[1])))
