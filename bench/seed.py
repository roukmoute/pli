"""Recrée le banc GreenMail et remplit la boîte d'Alice par IMAP."""

import argparse
import imaplib
import socket
import subprocess
from collections.abc import Callable
from dataclasses import dataclass
from datetime import UTC, datetime, timedelta
from email.message import EmailMessage
from email.policy import SMTP
from pathlib import Path


BENCH = Path(__file__).resolve().parent
COMPOSE = ["docker", "compose", "-f", str(BENCH / "docker-compose.yml")]
SIZES = {"vide": 0, "base": 20, "10k": 10_000, "100k": 100_000}
START = datetime(2025, 1, 6, 9, tzinfo=UTC)


def newsletter(message):
    message["List-ID"] = "Les nouvelles de l’atelier <lettre.recette.test>"
    message["List-Unsubscribe"] = "<mailto:desinscription@recette.test>"


def receipt(message):
    message.add_attachment(
        (BENCH / "fixtures/recu.pdf").read_bytes(),
        maintype="application", subtype="pdf", filename="recu.pdf",
    )


def illustrated_letter(message):
    message.add_alternative(
        '<p>Voici le timbre de l’atelier et la photo du jardin.</p>'
        '<img src="cid:timbre@recette.test" alt="Timbre de l’atelier">'
        '<img src="https://images.recette.test/jardin.png" alt="Jardin distant">',
        subtype="html",
    )
    html = message.get_body(preferencelist=("html",))
    html.add_related(
        (BENCH / "fixtures/timbre.png").read_bytes(), maintype="image", subtype="png",
        cid="<timbre@recette.test>", disposition="inline", filename="timbre.png",
    )


def measurement_file(message):
    message.add_attachment(
        bytes(range(256)) * 8192, maintype="application",
        subtype="octet-stream", filename="mesures.bin",
    )


@dataclass(frozen=True)
class Letter:
    sender: str
    subject: str
    body: str
    seen: bool = False
    references: tuple[int, ...] = ()
    enrich: Callable[[EmailMessage], None] | None = None


# Les numéros dans les références sont les positions des lettres, à partir de 1.
SAMPLES = (
    Letter("nouvelle", "Premier contact", "Je découvre votre atelier."),
    Letter("bob", "Plan de montage", "Voici le plan que nous avions relu.", seen=True),
    Letter("bob", "Re: Plan de montage", "Le montage est terminé.", references=(2,)),
    Letter("lettre", "La lettre de janvier", "Ce mois-ci, visite des serres.", enrich=newsletter),
    Letter("caisse", "Reçu de la librairie", "Le reçu de 12,00 EUR est joint.", enrich=receipt),
    Letter("carol", "Le jardin en images", "Un timbre et une photo du jardin.", enrich=illustrated_letter),
    Letter("carol", "Été à 京都", "À bientôt ! Καλημέρα · こんにちは · Добрый день."),
    Letter("bob", "Arrivée du premier lot", "Les pièces bleues sont arrivées."),
    Letter("carol", "Arrivée du second lot", "Les pièces rouges sont arrivées."),
    Letter("bob", "Réservation de salle", "La salle nord est réservée."),
    Letter("carol", "Réservation de salle", "La salle sud est réservée."),
    Letter("mesures", "Relevé de l’atelier", "Les mesures brutes sont jointes.", enrich=measurement_file),
)
TOPICS = ("Inventaire", "Jardinage", "Reliure", "Éclairage", "Réparation")


def letter_at(number, seed):
    if number <= len(SAMPLES):
        return SAMPLES[number - 1]
    thread, position = divmod(number - len(SAMPLES) - 1, 25)
    topic = TOPICS[(thread + seed) % len(TOPICS)]
    subject = f"{topic} - carnet {thread + 1}"
    return Letter(
        ("bob", "carol")[(number + seed) % 2],
        f"Re: {subject}" if position else subject,
        f"{topic}, relevé {number}. Lot de recette {seed}.",
        seen=position % 4 != 3,
        references=tuple(range(number - position, number)),
    )


def message_id(seed, number):
    return f"<pli-{seed}-{number}@recette.test>"


def messages(count, seed):
    for number in range(1, count + 1):
        letter = letter_at(number, seed)
        # Les deux livraisons ont la même date sur le message et sur le serveur.
        date = START + timedelta(minutes=8 if number == 9 else number)
        message = EmailMessage(policy=SMTP)
        message["Date"] = date
        message["From"] = f"{letter.sender}@recette.test"
        message["To"] = "alice@recette.test"
        message["Subject"] = letter.subject
        message["Message-ID"] = message_id(seed, number)
        if letter.references:
            parents = [message_id(seed, parent) for parent in letter.references]
            message["In-Reply-To"] = parents[-1]
            message["References"] = " ".join(parents)
        message.set_content(letter.body)
        if letter.enrich:
            letter.enrich(message)
            # La sérialisation MIME choisirait sinon des séparateurs aléatoires.
            for index, part in enumerate(message.walk()):
                if part.is_multipart():
                    part.set_boundary(f"pli-{seed}-{number}-part-{index}")
        yield date, letter.seen, message.as_bytes()


def require_ok(response):
    status, details = response
    if status != "OK":
        raise RuntimeError(f"GreenMail a répondu {status} {details!r}")
    return details


def populate(port, count, seed):
    with imaplib.IMAP4("127.0.0.1", port, timeout=30) as mailbox:
        # APPEND envoie le littéral et sa fin séparément, même sur la boucle locale.
        mailbox.socket().setsockopt(socket.IPPROTO_TCP, socket.TCP_NODELAY, 1)
        mailbox.login("alice@recette.test", "alice-test")
        if count:
            for folder in ("Atelier", "Gesendet", "Papierkorb"):
                require_ok(mailbox.create(folder))
                require_ok(mailbox.subscribe(folder))
        for number, (date, seen, raw) in enumerate(messages(count, seed), start=1):
            require_ok(mailbox.append("INBOX", r"(\Seen)" if seen else None, date, raw))
            if number % 10_000 == 0:
                print(f"{number}/{count} messages", flush=True)
        total = int(require_ok(mailbox.select("INBOX", readonly=True))[0])
        if total != count:
            raise RuntimeError(f"INBOX contient {total} messages, {count} attendus")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("dataset", choices=SIZES)
    parser.add_argument("--seed", type=int, default=42)
    args = parser.parse_args()
    try:
        subprocess.run(
            [*COMPOSE, "up", "--force-recreate", "--wait", "--wait-timeout", "90"],
            check=True,
        )
        address = subprocess.check_output([*COMPOSE, "port", "greenmail", "3143"], text=True)
        count = SIZES[args.dataset]
        populate(int(address.rsplit(":", 1)[1]), count, args.seed)
    except (Exception, KeyboardInterrupt):
        subprocess.run([*COMPOSE, "down"], check=False)
        raise
    print(f"INBOX d’Alice contient {count} messages (jeu {args.dataset}, graine {args.seed}).")


if __name__ == "__main__":
    main()
