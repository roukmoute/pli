"""Recette APS sur une boîte GreenMail jetable. Suspend un lot IMAP après sa première réponse."""

import asyncio
import imaplib
import os
import re
import smtplib
import ssl
import subprocess
import sys
from email.message import EmailMessage
from pathlib import Path
from uuid import uuid4


HOST = "127.0.0.1"
IMAPS = int(os.environ.get("PLI_APS_IMAPS_PORT", 3993))
SMTPS = int(os.environ.get("PLI_APS_SMTPS_PORT", 3465))


def tls_client():
    # Certificat autosigné du banc, exclusivement sur la boucle locale.
    context = ssl.SSLContext(ssl.PROTOCOL_TLS_CLIENT)
    context.check_hostname = False
    context.verify_mode = ssl.CERT_NONE
    return context


def deliver(address, subjects, *, sender="L’Atelier <atelier@recette.test>", thread=False):
    with smtplib.SMTP_SSL(HOST, SMTPS, context=tls_client(), timeout=8) as smtp:
        smtp.login("alice@recette.test", "alice-test")
        parent = None
        for subject in subjects:
            message = EmailMessage()
            message["From"] = sender
            message["To"] = address
            message["Subject"] = subject
            message["Message-ID"] = f"<{uuid4().hex}@recette.test>"
            if thread:
                if parent:
                    message["In-Reply-To"] = parent
                    message["References"] = parent
                parent = message["Message-ID"]
            body = f"Lettre de l’atelier. {subject}."
            if subject == "Archive B":
                body += "\n" + "Ligne de recette.\n" * 350
            message.set_content(body)
            smtp.send_message(message)


def unread(address):
    with imaplib.IMAP4_SSL(HOST, IMAPS, ssl_context=tls_client(), timeout=8) as mailbox:
        mailbox.login(address, address)
        mailbox.select("INBOX", readonly=True)
        status, result = mailbox.uid("SEARCH", None, "UNSEEN")
        if status != "OK":
            raise RuntimeError("Lecture des indicateurs IMAP refusée.")
        return len(result[0].split())


async def forward_commands(reader, writer):
    while data := await reader.read(8192):
        writer.write(data)
        await writer.drain()


async def serve(directory, pages=False, *, address=None, subjects=None):
    address = address or f"tri-{uuid4().hex}@recette.test"
    if subjects is None:
        subjects = [f"Archive {uid}" for uid in range(1, 52)] if pages else ["Archive A", "Archive B"]
    deliver(address, subjects)
    key, cert = (directory / name for name in ("banc.key", "banc.pem"))
    subprocess.run(
        ["openssl", "req", "-x509", "-newkey", "ec", "-pkeyopt", "ec_paramgen_curve:P-256",
         "-noenc", "-subj", "/CN=localhost", "-days", "1", "-keyout", key, "-out", cert],
        check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
    )
    server_tls = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
    server_tls.load_cert_chain(cert, key)
    paused = False

    async def replies(reader, writer):
        nonlocal paused
        fetched = 0
        while line := await reader.readline():
            if re.match(rb"^\* \d+ FETCH \(", line):
                fetched += 1
                if not pages and fetched == 2 and not paused:
                    paused = True
                    await asyncio.to_thread(deliver, address, ["Arrivée A", "Arrivée B"])
                    print("livré", flush=True)
                    # La première réponse a été transmise. Le redémarrage ferme ce flux.
                    await asyncio.Future()
            writer.write(line)
            await writer.drain()

    async def page_commands(reader, writer):
        nonlocal paused
        while line := await reader.readline():
            if not paused and re.search(rb"\bUID FETCH 1 \(UID ENVELOPE\b", line, re.I):
                # Le lot de 50 en-têtes est validé. Retenir le dernier UID jusqu’à la reprise.
                paused = True
                print("lot suspendu", flush=True)
                await asyncio.Future()
            writer.write(line)
            await writer.drain()

    async def relay(client, downstream):
        remote, upstream = await asyncio.open_connection(HOST, IMAPS, ssl=tls_client())
        commands = page_commands if pages else forward_commands
        tasks = [asyncio.create_task(commands(client, upstream)),
                 asyncio.create_task(replies(remote, downstream))]
        try:
            await asyncio.wait(tasks, return_when=asyncio.FIRST_COMPLETED)
        finally:
            for task in tasks:
                task.cancel()
            await asyncio.gather(*tasks, return_exceptions=True)
            upstream.close()
            downstream.close()

    async with await asyncio.start_server(relay, HOST, 0, ssl=server_tls) as server:
        print(server.sockets[0].getsockname()[1], address, flush=True)
        await server.serve_forever()


if __name__ == "__main__":
    if sys.argv[1] == "--unread":
        print(unread(sys.argv[2]))
    else:
        asyncio.run(serve(Path(sys.argv[1]), "--pages" in sys.argv[2:]))
