from email.parser import BytesParser
from email.policy import default
from unittest import TestCase

from seed import messages


class BaseMailTest(TestCase):
    def test_la_graine_reproduit_dates_drapeaux_et_octets_mime(self):
        first = list(messages(20, 42))
        self.assertEqual(first, list(messages(20, 42)))
        self.assertNotEqual(first[0], next(messages(1, 43)))

    def test_les_fils_se_distinguent_des_objets_et_dates_identiques(self):
        rows = list(messages(11, 42))
        mail = [BytesParser(policy=default).parsebytes(raw) for _, _, raw in rows]
        self.assertTrue(rows[1][1])
        self.assertFalse(rows[2][1])
        self.assertEqual(mail[2]["In-Reply-To"], mail[1]["Message-ID"])
        self.assertEqual(mail[2]["References"], mail[1]["Message-ID"])
        self.assertEqual(rows[7][0], rows[8][0])
        self.assertEqual(mail[7]["Date"], mail[8]["Date"])
        self.assertEqual(mail[9]["Subject"], mail[10]["Subject"])
        self.assertNotEqual(mail[9]["Message-ID"], mail[10]["Message-ID"])
        for independent in mail[9:11]:
            self.assertIsNone(independent["References"])
            self.assertIsNone(independent["In-Reply-To"])
