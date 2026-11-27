"""Copie quelques lettres du jeu base dans une boîte GreenMail distincte par scénario."""

import imaplib
import sys
from email import policy
from email.parser import BytesParser
from uuid import uuid4

from screener import HOST, IMAPS, deliver, tls_client
from seed import messages, require_ok


def preparer(port):
    adresse = f"lecture-{uuid4().hex}@recette.test"
    deliver(adresse, ["Bienvenue au jardin"])
    with imaplib.IMAP4_SSL(HOST, IMAPS, ssl_context=tls_client(), timeout=8) as boite:
        boite.login(adresse, adresse)
        for numero, (date, lu, raw) in enumerate(messages(7, 42), start=1):
            if numero in (2, 3, 5, 6, 7):
                if numero == 6:
                    message = BytesParser(policy=policy.SMTP).parsebytes(raw)
                    html = message.get_body(preferencelist=("html",))
                    html.set_content(html.get_content().replace(
                        "https://images.recette.test", f"http://127.0.0.1:{port}"
                    ), subtype="html")
                    raw = message.as_bytes()
                require_ok(boite.append("INBOX", r"(\Seen)" if lu else None, date, raw))
    return adresse


if __name__ == "__main__":
    print(preparer(int(sys.argv[1])))
