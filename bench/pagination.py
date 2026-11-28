"""Prépare une boîte GreenMail de vingt fils, dont dix-neuf reçus à la même date."""

import imaplib
import sys
from email.message import EmailMessage
from email.policy import SMTP
from uuid import uuid4

from screener import HOST, IMAPS, deliver, tls_client
from seed import START, require_ok

FILS_DE_MEME_DATE = 19


def preparer():
    adresse = f"pages-{uuid4().hex}@recette.test"
    # La première lettre crée la boîte. Son expéditeur écrira de nouveau pendant la lecture.
    deliver(adresse, ["Bienvenue à l’atelier"])
    with imaplib.IMAP4_SSL(HOST, IMAPS, ssl_context=tls_client(), timeout=8) as boite:
        boite.login(adresse, adresse)
        require_ok(boite.create("Atelier"))
        require_ok(boite.subscribe("Atelier"))
        for numero in range(1, FILS_DE_MEME_DATE + 1):
            message = EmailMessage(policy=SMTP)
            message["Date"] = START
            message["From"] = "bob@recette.test"
            message["To"] = adresse
            message["Subject"] = f"Relevé {numero:02}"
            message["Message-ID"] = f"<pages-{numero:02}-{uuid4().hex}@recette.test>"
            message.set_content(f"Relevé numéro {numero} de l’atelier.")
            require_ok(boite.append("INBOX", None, START, message.as_bytes()))
    return adresse


if __name__ == "__main__":
    if sys.argv[1:2] == ["--arrivee"]:
        deliver(sys.argv[2], ["Arrivée du soir"])
    else:
        print(preparer())
