-- Dossier choisi par l’utilisateur quand la messagerie n’annonce aucun dossier d’archives.
CREATE TABLE archive_choices (
    account_id INTEGER PRIMARY KEY REFERENCES accounts(id) ON DELETE CASCADE,
    folder_id INTEGER NOT NULL,
    FOREIGN KEY (account_id, folder_id) REFERENCES folders(account_id, id) ON DELETE CASCADE
) STRICT;

-- Le choix de l’utilisateur l’emporte sur le rôle découvert.
CREATE VIEW archive_folders AS
SELECT a.id AS account_id,
       coalesce((SELECT c.folder_id FROM archive_choices c WHERE c.account_id=a.id),
                (SELECT f.id FROM folders f WHERE f.account_id=a.id AND f.role='archive'
                 ORDER BY f.remote_path LIMIT 1)) AS folder_id
FROM accounts a;

-- Une lettre en cours de déplacement quitte les listes sans quitter la base :
-- un refus la rend, une confirmation la rattache à son dossier d’arrivée.
ALTER TABLE messages ADD COLUMN moving INTEGER NOT NULL DEFAULT 0 CHECK (moving IN (0,1));

DROP VIEW routed_messages;
CREATE VIEW routed_messages AS
SELECT m.*, d.state AS screening_state,
       CASE WHEN f.role='inbox' AND m.uidvalidity=b.uidvalidity THEN
           CASE WHEN d.state='accepte' THEN d.destination
                WHEN d.state IS NULL AND m.uid<b.uidnext THEN 'reception' END
       END AS destination
FROM messages m
JOIN folders f ON f.account_id=m.account_id AND f.id=m.folder_id AND f.uidvalidity=m.uidvalidity
JOIN inbox_markers b ON b.account_id=m.account_id
LEFT JOIN sender_decisions d ON d.account_id=m.account_id AND d.address=m.sender_address
    AND f.role='inbox' AND m.uidvalidity=b.uidvalidity
    AND (m.uid>=b.uidnext OR d.state!='en_attente')
WHERE NOT m.moving;

DROP VIEW screening_letters;
CREATE VIEW screening_letters AS
SELECT m.account_id, m.sender_address, m.uid, m.uidvalidity, m.thread_id, m.sender_name,
       m.subject, m.snippet, m.received_at,
       NOT EXISTS(SELECT 1 FROM json_each(m.flags) WHERE lower(value)='\seen') AS unread,
       EXISTS(SELECT 1 FROM json_each(m.flags) WHERE lower(value)='\flagged') AS starred,
       m.has_attachments AS attached,
       EXISTS(SELECT 1 FROM json_each(m.flags) WHERE lower(value)='\answered') AS answered,
       EXISTS(SELECT 1 FROM json_each(m.flags) WHERE lower(value)='$forwarded') AS forwarded
FROM messages m
JOIN folders f ON f.id=m.folder_id AND f.uidvalidity=m.uidvalidity AND f.role='inbox'
JOIN inbox_markers b ON b.account_id=m.account_id AND b.uidvalidity=m.uidvalidity
WHERE NOT m.moving;

-- Un genre d’opération modifie un drapeau ou déplace des lettres vers un dossier.
CREATE TABLE operation_kinds (
    kind TEXT PRIMARY KEY,
    flag TEXT,
    expected INTEGER CHECK (expected IN (0,1)),
    CHECK ((flag IS NULL) = (expected IS NULL))
) STRICT, WITHOUT ROWID;
INSERT INTO operation_kinds VALUES
    ('etoile','\Flagged',1),('sans_etoile','\Flagged',0),
    ('lu','\Seen',1),('non_lu','\Seen',0),
    ('archiver',NULL,NULL);

-- Les opérations sont recopiées dans des tables qui acceptent les déplacements.
CREATE TABLE operations_010 (
    seq INTEGER PRIMARY KEY,
    account_id INTEGER NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    id TEXT NOT NULL,
    kind TEXT NOT NULL REFERENCES operation_kinds(kind),
    -- Dossier d’arrivée d’un déplacement, fixé à l’enregistrement.
    destination INTEGER REFERENCES folders(id) ON DELETE SET NULL,
    state TEXT NOT NULL DEFAULT 'en_attente' CHECK (state IN ('en_attente','confirmee','refusee')),
    UNIQUE (account_id, id)
) STRICT;
INSERT INTO operations_010(seq,account_id,id,kind,state)
    SELECT seq,account_id,id,kind,state FROM operations;

CREATE TABLE operation_targets_010 (
    operation INTEGER NOT NULL REFERENCES operations_010(seq) ON DELETE CASCADE,
    position INTEGER NOT NULL,
    target TEXT NOT NULL,
    PRIMARY KEY (operation, position)
) STRICT;
INSERT INTO operation_targets_010 SELECT operation,position,target FROM operation_targets;

CREATE TABLE operation_messages_010 (
    operation INTEGER NOT NULL,
    position INTEGER NOT NULL,
    folder_id INTEGER NOT NULL,
    uidvalidity INTEGER NOT NULL,
    uid INTEGER NOT NULL,
    previous INTEGER NOT NULL CHECK (previous IN (0,1)),
    -- Commande de déplacement déjà envoyée : la lettre a pu partir avant une coupure.
    attempted INTEGER NOT NULL DEFAULT 0 CHECK (attempted IN (0,1)),
    result TEXT NOT NULL DEFAULT 'en_attente' CHECK (result IN ('en_attente','confirme','refuse','absent')),
    PRIMARY KEY (operation, position, folder_id, uidvalidity, uid),
    FOREIGN KEY (operation, position) REFERENCES operation_targets_010 ON DELETE CASCADE
) STRICT;
INSERT INTO operation_messages_010(operation,position,folder_id,uidvalidity,uid,previous,result)
    SELECT operation,position,folder_id,uidvalidity,uid,previous,result FROM operation_messages;

DROP VIEW operation_results;
DROP TABLE operation_messages;
DROP TABLE operation_targets;
DROP TABLE operations;
ALTER TABLE operations_010 RENAME TO operations;
ALTER TABLE operation_targets_010 RENAME TO operation_targets;
ALTER TABLE operation_messages_010 RENAME TO operation_messages;
CREATE INDEX operations_pending ON operations(account_id, seq) WHERE state='en_attente';
CREATE INDEX operation_letters ON operation_messages(folder_id, uidvalidity, uid, operation);

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
