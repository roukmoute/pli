CREATE TABLE folders (
    id INTEGER PRIMARY KEY,
    account_id INTEGER NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    remote_path TEXT NOT NULL,
    delimiter TEXT,
    role TEXT CHECK (role IN ('inbox', 'sent', 'drafts', 'junk', 'trash', 'archive')),
    uidvalidity INTEGER CHECK (uidvalidity BETWEEN 1 AND 4294967295),
    uidnext INTEGER CHECK (uidnext BETWEEN 1 AND 4294967295),
    hidden INTEGER NOT NULL DEFAULT 0 CHECK (hidden IN (0, 1)),
    UNIQUE (account_id, remote_path)
) STRICT;

CREATE TABLE inbox_markers (
    account_id INTEGER PRIMARY KEY REFERENCES accounts(id) ON DELETE CASCADE,
    uidvalidity INTEGER NOT NULL CHECK (uidvalidity BETWEEN 1 AND 4294967295),
    uidnext INTEGER NOT NULL CHECK (uidnext BETWEEN 1 AND 4294967295)
) STRICT;
