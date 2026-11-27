import imaplib
import io
import os
import unittest
from contextlib import nullcontext, redirect_stdout
from pathlib import Path
from tempfile import TemporaryDirectory
from unittest.mock import Mock, patch

from lecture_imap import Echec, compteurs
from recette_lecture import Recette, identifiants, main


class RecetteLectureTest(unittest.TestCase):
    def test_le_secret_est_lu_litteralement(self):
        with TemporaryDirectory() as dossier:
            fichier = Path(dossier) / "identifiants"
            fichier.write_text("export RECETTE_IMAP_USER=alice@recette.test\n"
                               "RECETTE_IMAP_PASSWORD='avec un # et = et $(texte)'\n")
            self.assertEqual(identifiants(fichier), ("alice@recette.test", "avec un # et = et $(texte)"))
            fichier.write_text("RECETTE_IMAP_PASSWORD=valeur-fictive\n")
            with self.assertRaises(Echec) as erreur:
                identifiants(fichier)
            self.assertNotIn("valeur-fictive", str(erreur.exception))

    def test_les_dossiers_a_espaces_et_les_litteraux_restent_distincts(self):
        boite = Mock()
        boite.list.return_value = ("OK", [b'() "/" INBOX', b'(\\Noselect) "/" Groupe',
                                        b'() "/" "Un dossier"', (b'() "/" {3}', b'A"B'), b''])
        boite.status.return_value = ("OK", [b'"nom" (UIDNEXT 9 MESSAGES 4 UIDVALIDITY 2 UNSEEN 3)'])
        etats = compteurs(boite)
        self.assertEqual(set(etats), {b"INBOX", b"Un dossier", b'A"B'})
        self.assertTrue(all(e["UNSEEN"] == 3 for e in etats.values()))
        self.assertEqual([c.args[0] for c in boite.status.call_args_list],
                         [b'"INBOX"', b'"Un dossier"', b'"A\\"B"'])

    def test_le_temoin_compare_les_non_lus_meme_apres_un_echec(self):
        recette = Recette()
        api = Mock()
        api.connecter.side_effect = imaplib.IMAP4.error("détail privé fictif")
        avant = {b"INBOX": {"UIDNEXT": 3, "UIDVALIDITY": 2, "UNSEEN": 1, "MESSAGES": 2}}
        with patch("recette_lecture.capacites", return_value={}), patch(
                "recette_lecture.compteurs", side_effect=[avant, avant]) as temoin:
            with self.assertRaises(imaplib.IMAP4.error):
                recette.observer(Mock(), nullcontext, api, Mock(), "alice@recette.test", "fictif")
        self.assertEqual(temoin.call_count, 2)
        self.assertEqual(recette.resultats["Non-lus avant et après"][0], "OK")
        self.assertEqual(recette.resultats["Rattachement dans Pli"][0], "Échec")
        self.assertNotIn("privé fictif", str(recette.resultats))

    def test_la_ci_sarrete_avant_de_lire_le_fichier(self):
        sortie = io.StringIO()
        with patch.dict(os.environ, CI="1"), patch("recette_lecture.identifiants") as lire, redirect_stdout(sortie):
            self.assertEqual(main(Path("inutilisé")), 1)
        lire.assert_not_called()
        self.assertIn("| Authentification IMAP | Non exécuté |", sortie.getvalue())
