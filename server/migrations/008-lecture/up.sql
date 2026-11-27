CREATE VIEW readable_messages AS
SELECT r.*, f.remote_path FROM routed_messages r
JOIN folders f ON f.id=r.folder_id
WHERE f.role IS NOT 'inbox' OR r.destination IS NOT NULL;

CREATE TABLE message_bodies (
    account_id INTEGER NOT NULL, folder_id INTEGER NOT NULL,
    uidvalidity INTEGER NOT NULL, uid INTEGER NOT NULL,
    render_version INTEGER NOT NULL, content TEXT NOT NULL,
    PRIMARY KEY (account_id, folder_id, uidvalidity, uid),
    FOREIGN KEY (account_id, folder_id, uidvalidity, uid)
        REFERENCES messages ON DELETE CASCADE
) STRICT;

CREATE TABLE message_images (
    account_id INTEGER NOT NULL, folder_id INTEGER NOT NULL,
    uidvalidity INTEGER NOT NULL, uid INTEGER NOT NULL, part INTEGER NOT NULL,
    mime TEXT NOT NULL, bytes BLOB NOT NULL,
    PRIMARY KEY (account_id, folder_id, uidvalidity, uid, part),
    FOREIGN KEY (account_id, folder_id, uidvalidity, uid)
        REFERENCES message_bodies ON DELETE CASCADE
) STRICT;
