CREATE TABLE server_profiles (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    imap_host TEXT NOT NULL,
    imap_port INTEGER NOT NULL CHECK (imap_port BETWEEN 1 AND 65535),
    imap_tls TEXT NOT NULL CHECK (imap_tls IN ('implicit', 'starttls')),
    smtp_host TEXT NOT NULL,
    smtp_port INTEGER NOT NULL CHECK (smtp_port BETWEEN 1 AND 65535),
    smtp_tls TEXT NOT NULL CHECK (smtp_tls IN ('implicit', 'starttls')),
    domains TEXT NOT NULL CHECK (json_type(domains) = 'array'),
    active INTEGER NOT NULL CHECK (active IN (0, 1)),
    local_test INTEGER NOT NULL CHECK (local_test IN (0, 1))
) STRICT;
