ALTER TABLE sender_decisions ADD COLUMN decided_at INTEGER;
CREATE INDEX decisions_history ON sender_decisions(account_id, state, address);

-- Avant la première décision, l’historique importé se lit en Réception.
-- Une décision explicite couvre ensuite tout le courrier de cette adresse dans INBOX.
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
    AND (m.uid>=b.uidnext OR d.state!='en_attente');

DROP TRIGGER reroute_sender;
CREATE TRIGGER reroute_sender AFTER INSERT ON list_reroute_sender BEGIN
    INSERT INTO list_refresh SELECT DISTINCT m.account_id,m.thread_id FROM messages m
      JOIN inbox_markers b ON b.account_id=m.account_id AND b.uidvalidity=m.uidvalidity
      JOIN folders f ON f.id=m.folder_id AND f.role='inbox' AND f.uidvalidity=m.uidvalidity
      WHERE m.account_id=NEW.account_id AND m.sender_address=NEW.address;
    INSERT INTO list_refresh VALUES(NEW.account_id,NEW.address);
    DELETE FROM list_reroute_sender WHERE rowid=NEW.rowid;
END;
CREATE TRIGGER decision_reconfirmed AFTER UPDATE OF revision ON sender_decisions
WHEN OLD.revision!=NEW.revision AND (OLD.state,OLD.destination) IS (NEW.state,NEW.destination) BEGIN
    UPDATE account_revisions SET revision=revision+1 WHERE account_id=NEW.account_id;
END;

-- L’arrivée d’une adresse en attente laisse l’historique à sa place.
-- Seuls les nouveaux UID déjà indexés peuvent alors changer de projection.
DROP TRIGGER list_decision_insert;
CREATE TRIGGER list_pending_insert AFTER INSERT ON sender_decisions WHEN NEW.state='en_attente' BEGIN
    INSERT INTO list_refresh SELECT DISTINCT m.account_id,m.thread_id FROM messages m
      JOIN inbox_markers b ON b.account_id=m.account_id AND b.uidvalidity=m.uidvalidity AND m.uid>=b.uidnext
      JOIN folders f ON f.id=m.folder_id AND f.role='inbox' AND f.uidvalidity=m.uidvalidity
      WHERE m.account_id=NEW.account_id AND m.sender_address=NEW.address;
    INSERT INTO list_refresh VALUES(NEW.account_id,NEW.address);
END;
CREATE TRIGGER list_decision_insert AFTER INSERT ON sender_decisions WHEN NEW.state!='en_attente' BEGIN
    INSERT INTO list_reroute_sender VALUES (NEW.account_id,NEW.address);
END;

INSERT INTO list_reroute_sender SELECT account_id,address FROM sender_decisions WHERE state!='en_attente';

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
JOIN inbox_markers b ON b.account_id=m.account_id AND b.uidvalidity=m.uidvalidity;
