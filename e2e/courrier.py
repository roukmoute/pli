"""Commandes du jalon Lecture, réservées au banc GreenMail isolé."""

import asyncio
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "bench"))
from screener import deliver, serve


ALICE = "alice@recette.test"

if __name__ == "__main__":
    action = sys.argv[1]
    if action == "historique":
        # Le jeu de base compte vingt messages. Le proxy suspend la requête du 51e,
        # après le premier lot de 50, jusqu’au redémarrage du service testé.
        anciens = [f"Bulletin de la bibliothèque {n:02}" for n in range(1, 32)]
        asyncio.run(serve(Path(sys.argv[2]), pages=True, address=ALICE, subjects=anciens))
    elif action == "correspondant":
        deliver(ALICE, ["Les épreuves sont prêtes", "Re: Les épreuves sont prêtes"],
                sender="La Relieuse <reliure@recette.test>", thread=True)
    elif action == "prospection":
        deliver(ALICE, ["Offre de démonstration"], sender="Prospection <prospection@recette.test>")
    else:
        raise SystemExit("Commande du banc inconnue.")
