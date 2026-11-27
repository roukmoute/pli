CREATE TABLE sender_decisions (
    account_id INTEGER NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    address TEXT NOT NULL,
    state TEXT NOT NULL DEFAULT 'en_attente' CHECK (state IN ('en_attente', 'accepte', 'refuse')),
    destination TEXT NOT NULL DEFAULT 'reception' CHECK (destination IN ('reception', 'flux', 'justificatifs')),
    revision INTEGER NOT NULL DEFAULT 1 CHECK (revision > 0),
    PRIMARY KEY (account_id, address)
) STRICT;

CREATE UNIQUE INDEX folders_account_id ON folders(account_id, id);

CREATE TABLE messages (
    account_id INTEGER NOT NULL,
    folder_id INTEGER NOT NULL,
    uidvalidity INTEGER NOT NULL CHECK (uidvalidity BETWEEN 1 AND 4294967295),
    uid INTEGER NOT NULL CHECK (uid BETWEEN 1 AND 4294967295),
    message_id TEXT,
    sender_address TEXT,
    sender_name TEXT,
    recipients TEXT NOT NULL,
    subject TEXT NOT NULL,
    sent_at INTEGER,
    received_at INTEGER NOT NULL,
    flags TEXT NOT NULL,
    size INTEGER NOT NULL CHECK (size >= 0),
    has_attachments INTEGER NOT NULL CHECK (has_attachments IN (0, 1)),
    snippet TEXT NOT NULL,
    thread_id TEXT NOT NULL,
    PRIMARY KEY (account_id, folder_id, uidvalidity, uid),
    FOREIGN KEY (account_id, folder_id) REFERENCES folders(account_id, id) ON DELETE CASCADE
) STRICT;

CREATE INDEX messages_by_sender ON messages(account_id, sender_address, uid DESC);
CREATE INDEX messages_by_thread ON messages(account_id, thread_id);

-- Un parent peut être cité avant que son propre en-tête soit importé.
CREATE TABLE thread_references (
    account_id INTEGER NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    message_id TEXT NOT NULL,
    thread_id TEXT NOT NULL,
    PRIMARY KEY (account_id, message_id)
) STRICT;
CREATE INDEX references_by_thread ON thread_references(account_id, thread_id);

CREATE VIEW routed_messages AS
SELECT m.*, d.state AS screening_state,
       CASE WHEN f.role = 'inbox' AND m.uidvalidity = b.uidvalidity THEN
           CASE WHEN m.uid < b.uidnext THEN 'reception'
                WHEN d.state = 'accepte' THEN d.destination END
       END AS destination
FROM messages m
JOIN folders f ON f.account_id = m.account_id AND f.id = m.folder_id AND f.uidvalidity = m.uidvalidity
JOIN inbox_markers b ON b.account_id = m.account_id
LEFT JOIN sender_decisions d ON d.account_id = m.account_id AND d.address = m.sender_address
    AND f.role = 'inbox' AND m.uidvalidity = b.uidvalidity AND m.uid >= b.uidnext;
