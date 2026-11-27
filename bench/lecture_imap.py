"""Observations IMAP en lecture seule pour la recette de Pli."""

from email import policy
from email.parser import BytesParser
from time import monotonic, sleep
import re


CAPACITES = ("IDLE", "CONDSTORE", "QRESYNC", "SPECIAL-USE", "MOVE", "UIDPLUS")


class Echec(Exception):
    """Message fixe publiable. Les réponses brutes du serveur restent en mémoire."""


def donnees(reponse):
    statut, lignes = reponse
    if statut != "OK":
        raise Echec("La messagerie a refusé une commande de lecture.")
    return lignes


def capacites(boite):
    mots = b" ".join(donnees(boite.capability())).upper().split()
    return {nom: nom.encode() in mots for nom in CAPACITES}


def nom_imap(nom):
    return b'"' + nom.replace(b"\\", b"\\\\").replace(b'"', b'\\"') + b'"'


def dossiers(boite):
    motif = rb'^\(([^)]*)\)\s+(?:"(?:[^"\\]|\\.)*"|NIL)\s+(.+)$'
    for entree in donnees(boite.list()):
        ligne = entree[0] if isinstance(entree, tuple) else entree
        if not ligne or ligne == b")":
            continue
        info = re.match(motif, ligne, flags=re.I)
        if not info:
            raise Echec("Une réponse de découverte des dossiers est illisible.")
        if b"\\NOSELECT" in info[1].upper().split():
            continue
        nom = info[2]
        if isinstance(entree, tuple):
            nom = entree[1]
        elif nom.startswith(b'"') and nom.endswith(b'"'):
            nom = re.sub(rb'\\(.)', rb'\1', nom[1:-1])
        yield nom


def compteurs(boite):
    etats = {}
    for nom in dossiers(boite):
        lignes = donnees(boite.status(nom_imap(nom), "(UNSEEN UIDVALIDITY UIDNEXT MESSAGES)"))
        info = lignes[0].rsplit(b"(", 1)[-1]
        valeurs = {cle.decode(): int(n) for cle, n in re.findall(rb"(\w+) (\d+)", info)}
        if not {"UNSEEN", "UIDVALIDITY", "UIDNEXT", "MESSAGES"} <= valeurs.keys():
            raise Echec("Le serveur a fourni des compteurs incomplets.")
        etats[nom if nom.upper() != b"INBOX" else b"INBOX"] = valeurs
    if b"INBOX" not in etats:
        raise Echec("La Réception est absente de la découverte IMAP.")
    return etats


def types_possibles(boite, uid, avec_piece):
    lignes = donnees(boite.uid("FETCH", str(uid), "(BODYSTRUCTURE)"))
    structure = b" ".join(l[0] if isinstance(l, tuple) else l for l in lignes if l)
    types = set()
    if re.search(rb'"TEXT"\s+"HTML"', structure, re.I):
        types.add("HTML")
    if re.search(rb'"TEXT"\s+"PLAIN"', structure, re.I):
        types.add("Texte")
    if avec_piece:
        types.add("Pièces jointes")
    return types


def source(boite, uid):
    for entree in donnees(boite.uid("FETCH", str(uid), "(UID BODY.PEEK[])")):
        if isinstance(entree, tuple):
            return BytesParser(policy=policy.default).parsebytes(entree[1])
    raise Echec("Un message a disparu pendant sa vérification.")


def contenu_concordant(message, contenu):
    verifies = set()
    corps = message.get_body(preferencelist=("html", "plain"))
    if corps and contenu["html"].strip():
        verifies.add({"text/html": "HTML", "text/plain": "Texte"}[corps.get_content_type()])
    pieces = {(p["nom"], p["mime"], p["taille"]) for p in contenu["pieces"]}
    for part in message.walk():
        if part.get_content_disposition() != "attachment" and not part.get_filename():
            continue
        octets = part.get_payload(decode=True)
        if octets is not None and (part.get_filename(), part.get_content_type(), len(octets)) in pieces:
            verifies.add("Pièces jointes")
            break
    return verifies


def verifier_corps(boite, api, db, resultats):
    manquants = {"Texte", "HTML", "Pièces jointes"}
    limite = monotonic() + 600
    # Les petits messages sont essayés en premier. Les structures ne contiennent pas le corps.
    reperes = db.execute("""SELECT id, remote_path, uidvalidity, uidnext FROM folders
            WHERE uidvalidity IS NOT NULL ORDER BY role IS 'inbox' DESC, id""").fetchall()
    for dossier, chemin, generation, prochain in reperes:
        if not manquants:
            break
        if prochain > 1:
            while not db.execute("""SELECT 1 FROM folder_imports WHERE folder_id=? AND uidvalidity=?
                                   AND indexed=expected""", (dossier, generation)).fetchone():
                if monotonic() >= limite:
                    raise Echec("L’indexation des dossiers à échantillonner est incomplète.")
                sleep(.2)
        examine = False
        for uid, piece in db.execute("""
                SELECT uid, has_attachments FROM readable_messages
                WHERE folder_id=? AND uidvalidity=? ORDER BY size, uid""", (dossier, generation)):
            if not manquants:
                break
            if manquants == {"Pièces jointes"} and not piece:
                continue
            if not examine:
                donnees(boite.select(nom_imap(chemin.encode()), readonly=True))
                _, validite = boite.response("UIDVALIDITY")
                if not validite or validite[0] != str(generation).encode():
                    raise Echec("La génération du dossier a changé pendant le contrôle.")
                examine = True
            possibles = types_possibles(boite, uid, piece) & manquants
            if not possibles:
                continue
            message = source(boite, uid)
            principal = message.get_body(preferencelist=("html", "plain"))
            format_corps = principal.get_content_type() if principal else None
            utiles = {k for k, mime in (("HTML", "text/html"), ("Texte", "text/plain"))
                      if k in possibles and format_corps == mime}
            if not utiles and "Pièces jointes" not in possibles:
                continue
            debut = monotonic()
            reponse = api.get(f"/api/v1/messages/{dossier}-{generation}-{uid}")
            if reponse["etat"] != "disponible":
                raise Echec("Le service n’a pas pu lire un corps sélectionné.")
            duree = (monotonic() - debut) * 1000
            for cle in contenu_concordant(message, reponse["contenu"]) & manquants:
                preuve = ("type, nom et taille confirmés" if cle == "Pièces jointes"
                          else "type MIME confirmé et corps affichable")
                resultats[cle] = ("OK", f"{preuve}. Lecture HTTP en {duree:.0f} ms.")
                manquants.remove(cle)
    for cle in manquants:
        resultats[cle] = ("Non vérifié", "Aucun exemple correspondant dans les messages indexés.")
