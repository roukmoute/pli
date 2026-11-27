CREATE TABLE users (
    id INTEGER PRIMARY KEY
) STRICT;

CREATE TABLE accounts (
    id INTEGER PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users(id),
    address TEXT NOT NULL,
    server_profile_id INTEGER NOT NULL REFERENCES server_profiles(id),
    encrypted_secret BLOB NOT NULL,
    UNIQUE (server_profile_id, address)
) STRICT;

-- Le futur rattachement de plusieurs boîtes pourra lever cette contrainte.
CREATE UNIQUE INDEX accounts_one_per_user ON accounts(user_id);

CREATE TABLE sessions (
    token_hash BLOB PRIMARY KEY,
    account_id INTEGER NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    expires_at INTEGER NOT NULL
) STRICT;

CREATE INDEX sessions_expiration ON sessions(expires_at);
