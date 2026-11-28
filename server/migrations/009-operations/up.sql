-- Une action locale est enregistrée avec la projection qu’elle modifie, puis
-- appliquée chez l’hébergeur dans son ordre d’arrivée.
CREATE TABLE operations (
    seq INTEGER PRIMARY KEY,
    account_id INTEGER NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    id TEXT NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('etoile','sans_etoile','lu','non_lu')),
    flag TEXT NOT NULL GENERATED ALWAYS AS (CASE WHEN kind IN ('lu','non_lu') THEN '\Seen' ELSE '\Flagged' END) VIRTUAL,
    expected INTEGER NOT NULL GENERATED ALWAYS AS (kind IN ('etoile','lu')) VIRTUAL,
    state TEXT NOT NULL DEFAULT 'en_attente' CHECK (state IN ('en_attente','confirmee','refusee')),
    UNIQUE (account_id, id)
) STRICT;
CREATE INDEX operations_pending ON operations(account_id, seq) WHERE state='en_attente';

-- Le rang d’une cible fixe l’ordre du résultat rendu par élément.
CREATE TABLE operation_targets (
    operation INTEGER NOT NULL REFERENCES operations(seq) ON DELETE CASCADE,
    position INTEGER NOT NULL,
    target TEXT NOT NULL,
    PRIMARY KEY (operation, position)
) STRICT;

-- Une ligne par lettre atteinte. L’état d’avant sert à rétablir un refus.
CREATE TABLE operation_messages (
    operation INTEGER NOT NULL,
    position INTEGER NOT NULL,
    folder_id INTEGER NOT NULL,
    uidvalidity INTEGER NOT NULL,
    uid INTEGER NOT NULL,
    previous INTEGER NOT NULL CHECK (previous IN (0,1)),
    result TEXT NOT NULL DEFAULT 'en_attente' CHECK (result IN ('en_attente','confirme','refuse','absent')),
    PRIMARY KEY (operation, position, folder_id, uidvalidity, uid),
    FOREIGN KEY (operation, position) REFERENCES operation_targets ON DELETE CASCADE
) STRICT;
CREATE INDEX operation_letters ON operation_messages(folder_id, uidvalidity, uid, operation);

-- Un fil dont une partie seulement des lettres est confirmée est partiel.
CREATE VIEW operation_results AS
SELECT t.operation, t.position, t.target,
       CASE WHEN count(m.uid)=0 THEN 'absent'
            WHEN sum(m.result='en_attente')>0 THEN 'en_attente'
            WHEN sum(m.result='confirme')=count(m.uid) THEN 'confirme'
            WHEN sum(m.result='confirme')>0 THEN 'partiel'
            WHEN sum(m.result='refuse')>0 THEN 'refuse'
            ELSE 'absent' END AS result
FROM operation_targets t
LEFT JOIN operation_messages m ON m.operation=t.operation AND m.position=t.position
GROUP BY t.operation, t.position;
