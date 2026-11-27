use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;

use super::secret::Token;
use crate::{Error, admin};

#[derive(Serialize)]
pub(crate) struct CompteAuthentifie {
    pub account_id: i64,
    pub address: String,
    pub expires_at: i64,
}

pub(super) struct Mailbox {
    pub address: String,
    pub domain: String,
}

impl Mailbox {
    pub fn parse(input: &str) -> Option<Self> {
        let (local, domain) = input.trim().split_once('@')?;
        if local.is_empty()
            || domain.is_empty()
            || domain.contains('@')
            || input
                .trim()
                .chars()
                .any(|c| c.is_control() || c.is_whitespace())
        {
            return None;
        }
        let domain = domain.to_ascii_lowercase();
        Some(Self {
            address: format!("{local}@{domain}"),
            domain,
        })
    }
}

fn save_account(
    db: &Connection,
    address: &str,
    profile: i64,
    encrypted: &[u8],
) -> rusqlite::Result<i64> {
    let fields = params![profile, address, encrypted];
    if let Some(id) = db.query_row(
        "UPDATE accounts SET encrypted_secret=?3 WHERE server_profile_id=?1 AND address=?2 RETURNING id",
        fields, |row| row.get(0),
    ).optional()? {
        return Ok(id);
    }
    db.execute("INSERT INTO users DEFAULT VALUES", [])?;
    db.query_row(
        "INSERT INTO accounts(user_id, server_profile_id, address, encrypted_secret)
         VALUES(last_insert_rowid(), ?1, ?2, ?3) RETURNING id",
        fields,
        |row| row.get(0),
    )
}

pub(super) fn attach(
    db: &mut Connection,
    mailbox: &Mailbox,
    server: &admin::Server,
    encrypted: &[u8],
) -> Result<(i64, bool), Error> {
    let tx = db.transaction()?;
    // Le profil peut avoir changé pendant l’aller-retour IMAP.
    if admin::for_domain(&tx, &mailbox.domain)?.as_ref() != Some(server) {
        return Err("Le profil a changé pendant la connexion.".into());
    }
    let account_id = save_account(&tx, &mailbox.address, server.id, encrypted)?;
    let has_marker = tx.query_row(
        "SELECT EXISTS (SELECT 1 FROM inbox_markers WHERE account_id=?1)",
        [account_id],
        |r| r.get(0),
    )?;
    tx.commit()?;
    Ok((account_id, has_marker))
}

pub(super) fn open(
    db: &mut Connection,
    account_id: i64,
    address: String,
    token: &Token,
    previous: Option<Token>,
    ttl: u32,
) -> Result<CompteAuthentifie, Error> {
    let tx = db.transaction()?;
    if let Some(old) = previous {
        revoke(&tx, &old)?;
    }
    tx.execute("DELETE FROM sessions WHERE expires_at <= unixepoch()", [])?;
    let expires_at = tx.query_row(
        "INSERT INTO sessions(token_hash, account_id, expires_at)
         VALUES(?1, ?2, unixepoch() + ?3) RETURNING expires_at",
        params![token.hash().as_ref(), account_id, ttl],
        |row| row.get(0),
    )?;
    tx.commit()?;
    Ok(CompteAuthentifie {
        account_id,
        address,
        expires_at,
    })
}

pub(super) fn current(
    db: &Connection,
    token: &Token,
) -> rusqlite::Result<Option<CompteAuthentifie>> {
    db.query_row(
        "SELECT a.id, a.address, s.expires_at FROM sessions s
         JOIN accounts a ON a.id = s.account_id WHERE s.token_hash = ?1 AND s.expires_at > unixepoch()",
        [token.hash().as_ref()],
        |row| Ok(CompteAuthentifie { account_id: row.get(0)?, address: row.get(1)?, expires_at: row.get(2)? }),
    ).optional()
}

pub(super) fn revoke(db: &Connection, token: &Token) -> rusqlite::Result<usize> {
    db.execute(
        "DELETE FROM sessions WHERE token_hash=?1",
        [token.hash().as_ref()],
    )
}
