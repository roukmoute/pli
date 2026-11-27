-- Les lectures du webmail accèdent directement aux fils déjà classés.
CREATE TABLE account_revisions (
    account_id INTEGER PRIMARY KEY REFERENCES accounts(id) ON DELETE CASCADE,
    revision INTEGER NOT NULL DEFAULT 0
) STRICT;
INSERT INTO account_revisions(account_id) SELECT id FROM accounts;
CREATE TRIGGER revision_new_account AFTER INSERT ON accounts BEGIN
    INSERT INTO account_revisions(account_id) VALUES (NEW.id);
END;

CREATE TABLE folder_imports (
    folder_id INTEGER PRIMARY KEY REFERENCES folders(id) ON DELETE CASCADE,
    uidvalidity INTEGER NOT NULL, until_uid INTEGER NOT NULL,
    expected INTEGER NOT NULL, indexed INTEGER NOT NULL
) STRICT;
CREATE TRIGGER import_known AFTER INSERT ON folder_imports BEGIN
    UPDATE account_revisions SET revision=revision+1
        WHERE account_id=(SELECT account_id FROM folders WHERE id=NEW.folder_id);
END;
CREATE TRIGGER import_progress AFTER UPDATE ON folder_imports
WHEN (OLD.uidvalidity,OLD.until_uid,OLD.expected,OLD.indexed) IS NOT
     (NEW.uidvalidity,NEW.until_uid,NEW.expected,NEW.indexed) BEGIN
    UPDATE account_revisions SET revision=revision+1
        WHERE account_id=(SELECT account_id FROM folders WHERE id=NEW.folder_id);
END;

-- Une lettre du dossier INBOX ne contribue à une liste ordinaire qu’une fois
-- admise par routed_messages. Un dossier personnel garde sa propre destination.
CREATE VIEW list_members AS
SELECT r.account_id, r.thread_id AS item, 'dossier/' || r.folder_id AS scope,
       r.folder_id, r.uidvalidity, r.uid, r.received_at, r.flags, r.has_attachments
FROM routed_messages r JOIN folders f ON f.id=r.folder_id
WHERE f.role IS NOT 'inbox' OR r.destination IS NOT NULL
UNION ALL
SELECT account_id, thread_id, destination, folder_id, uidvalidity, uid, received_at, flags, has_attachments
FROM routed_messages WHERE destination IS NOT NULL
UNION ALL
SELECT account_id, sender_address, 'tri', folder_id, uidvalidity, uid, received_at, flags, has_attachments
FROM routed_messages WHERE screening_state='en_attente';

CREATE VIEW ranked_list_members AS
SELECT account_id, item, scope, folder_id, uidvalidity, uid, received_at,
       count(*) OVER members AS message_count,
       max(NOT EXISTS(SELECT 1 FROM json_each(flags) WHERE lower(value)='\seen')) OVER members AS unread,
       max(EXISTS(SELECT 1 FROM json_each(flags) WHERE lower(value)='\flagged')) OVER members AS starred,
       max(has_attachments) OVER members AS attached,
       max(EXISTS(SELECT 1 FROM json_each(flags) WHERE lower(value)='\answered')) OVER members AS answered,
       max(EXISTS(SELECT 1 FROM json_each(flags) WHERE lower(value)='$forwarded')) OVER members AS forwarded,
       row_number() OVER (members ORDER BY received_at DESC, folder_id DESC, uidvalidity DESC, uid DESC) AS position
FROM list_members WINDOW members AS (PARTITION BY account_id, item, scope);

CREATE VIEW current_list_threads AS
SELECT account_id, item, scope, folder_id, uidvalidity, uid, received_at,
       message_count, unread, starred, attached, answered, forwarded
FROM ranked_list_members WHERE position=1;

CREATE TABLE list_threads (
    account_id INTEGER NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    item TEXT NOT NULL, scope TEXT NOT NULL, folder_id INTEGER NOT NULL,
    uidvalidity INTEGER NOT NULL, uid INTEGER NOT NULL, received_at INTEGER NOT NULL,
    message_count INTEGER NOT NULL, unread INTEGER NOT NULL, starred INTEGER NOT NULL,
    attached INTEGER NOT NULL, answered INTEGER NOT NULL, forwarded INTEGER NOT NULL,
    PRIMARY KEY(account_id, item, scope)
) STRICT;
CREATE INDEX list_page ON list_threads(account_id, scope, received_at DESC, item DESC);
CREATE TABLE list_counts (
    account_id INTEGER NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    scope TEXT NOT NULL, total INTEGER NOT NULL, PRIMARY KEY(account_id, scope)
) STRICT;
CREATE TRIGGER count_new_thread AFTER INSERT ON list_threads BEGIN
    INSERT INTO list_counts VALUES (NEW.account_id, NEW.scope, 1)
    ON CONFLICT(account_id,scope) DO UPDATE SET total=total+1;
END;
CREATE TRIGGER count_removed_thread AFTER DELETE ON list_threads BEGIN
    UPDATE list_counts SET total=total-1 WHERE account_id=OLD.account_id AND scope=OLD.scope;
END;
INSERT INTO list_threads SELECT * FROM current_list_threads;

-- Ce point d’entrée SQL recalcule un fil (ou l’entrée de tri d’une adresse)
-- dans la transaction de l’écriture. La file est vide à la fin de chaque instruction.
CREATE TABLE list_refresh (account_id INTEGER NOT NULL, item TEXT NOT NULL) STRICT;
CREATE TRIGGER refresh_list_item AFTER INSERT ON list_refresh BEGIN
    DELETE FROM list_threads WHERE account_id=NEW.account_id AND item=NEW.item;
    INSERT INTO list_threads SELECT * FROM current_list_threads
        WHERE account_id=NEW.account_id AND item=NEW.item;
    UPDATE account_revisions SET revision=revision+1 WHERE account_id=NEW.account_id;
    DELETE FROM list_refresh WHERE rowid=NEW.rowid;
END;

CREATE TRIGGER list_message_insert AFTER INSERT ON messages BEGIN
    INSERT INTO list_refresh VALUES (NEW.account_id, NEW.thread_id);
    INSERT INTO list_refresh SELECT NEW.account_id, NEW.sender_address
        WHERE EXISTS (SELECT 1 FROM sender_decisions WHERE account_id=NEW.account_id
                      AND address=NEW.sender_address AND state='en_attente');
    UPDATE folder_imports SET indexed=min(expected,indexed+1)
        WHERE folder_id=NEW.folder_id AND uidvalidity=NEW.uidvalidity AND NEW.uid<until_uid;
END;
CREATE TRIGGER list_message_delete AFTER DELETE ON messages BEGIN
    INSERT INTO list_refresh VALUES (OLD.account_id, OLD.thread_id);
    INSERT INTO list_refresh SELECT OLD.account_id, OLD.sender_address
        WHERE EXISTS (SELECT 1 FROM list_threads WHERE account_id=OLD.account_id
                      AND item=OLD.sender_address AND scope='tri');
END;
CREATE TRIGGER list_message_update AFTER UPDATE ON messages BEGIN
    INSERT INTO list_refresh SELECT OLD.account_id, OLD.thread_id UNION SELECT NEW.account_id, NEW.thread_id;
    INSERT INTO list_refresh SELECT account_id,address FROM sender_decisions
        WHERE state='en_attente' AND (account_id,address) IN
              ((OLD.account_id,OLD.sender_address),(NEW.account_id,NEW.sender_address));
END;

-- Les changements de décision ou de génération affectent les fils concernés,
-- y compris ceux qui contenaient aussi un message admis d’un autre expéditeur.
CREATE TABLE list_reroute_sender (account_id INTEGER NOT NULL, address TEXT NOT NULL) STRICT;
CREATE TRIGGER reroute_sender AFTER INSERT ON list_reroute_sender BEGIN
    INSERT INTO list_refresh SELECT DISTINCT m.account_id,m.thread_id FROM messages m
      JOIN inbox_markers b ON b.account_id=m.account_id AND b.uidvalidity=m.uidvalidity AND m.uid>=b.uidnext
      JOIN folders f ON f.id=m.folder_id AND f.role='inbox'
      WHERE m.account_id=NEW.account_id AND m.sender_address=NEW.address;
    INSERT INTO list_refresh VALUES(NEW.account_id,NEW.address);
    DELETE FROM list_reroute_sender WHERE rowid=NEW.rowid;
END;
CREATE TRIGGER list_decision_insert AFTER INSERT ON sender_decisions BEGIN
    INSERT INTO list_reroute_sender VALUES (NEW.account_id,NEW.address);
END;
CREATE TRIGGER list_decision_delete AFTER DELETE ON sender_decisions BEGIN
    INSERT INTO list_reroute_sender VALUES (OLD.account_id,OLD.address);
END;
CREATE TRIGGER list_decision_update AFTER UPDATE OF state,destination ON sender_decisions
WHEN OLD.state IS NOT NEW.state OR OLD.destination IS NOT NEW.destination BEGIN
    INSERT INTO list_reroute_sender VALUES (NEW.account_id,NEW.address);
END;
CREATE TRIGGER list_folder_generation AFTER UPDATE OF role,uidvalidity ON folders
WHEN OLD.role IS NOT NEW.role OR OLD.uidvalidity IS NOT NEW.uidvalidity BEGIN
    INSERT INTO list_refresh SELECT account_id,thread_id FROM messages WHERE folder_id=NEW.id
        UNION SELECT account_id,sender_address FROM messages WHERE folder_id=NEW.id AND sender_address IS NOT NULL;
    DELETE FROM folder_imports WHERE folder_id=NEW.id AND uidvalidity IS NOT NEW.uidvalidity;
    UPDATE account_revisions SET revision=revision+1 WHERE account_id=NEW.account_id;
END;
CREATE TRIGGER list_folder_created AFTER INSERT ON folders BEGIN
    UPDATE account_revisions SET revision=revision+1 WHERE account_id=NEW.account_id;
END;
CREATE TRIGGER list_folder_removed AFTER DELETE ON folders BEGIN
    UPDATE account_revisions SET revision=revision+1 WHERE account_id=OLD.account_id;
END;
CREATE TRIGGER list_folder_empty AFTER UPDATE OF uidnext ON folders
WHEN (OLD.uidnext IS 1) != (NEW.uidnext IS 1) BEGIN
    UPDATE account_revisions SET revision=revision+1 WHERE account_id=NEW.account_id;
END;

CREATE TABLE list_reroute_account (account_id INTEGER NOT NULL) STRICT;
CREATE TRIGGER reroute_account AFTER INSERT ON list_reroute_account BEGIN
    INSERT INTO list_refresh SELECT account_id,thread_id FROM messages WHERE account_id=NEW.account_id
        UNION SELECT account_id,address FROM sender_decisions WHERE account_id=NEW.account_id;
    UPDATE account_revisions SET revision=revision+1 WHERE account_id=NEW.account_id;
    DELETE FROM list_reroute_account WHERE rowid=NEW.rowid;
END;
CREATE TRIGGER list_marker_insert AFTER INSERT ON inbox_markers BEGIN
    INSERT INTO list_reroute_account VALUES(NEW.account_id);
END;
CREATE TRIGGER list_marker_update AFTER UPDATE OF uidvalidity,uidnext ON inbox_markers
WHEN (OLD.uidvalidity,OLD.uidnext) IS NOT (NEW.uidvalidity,NEW.uidnext) BEGIN
    INSERT INTO list_reroute_account VALUES(NEW.account_id);
END;
CREATE TRIGGER list_marker_delete AFTER DELETE ON inbox_markers BEGIN
    INSERT INTO list_reroute_account VALUES(OLD.account_id);
END;
