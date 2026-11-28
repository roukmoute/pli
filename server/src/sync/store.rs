use rusqlite::{Connection, params};
use serde::Serialize;

use super::discovery::{Discovery, display_path};
use crate::Error;

pub(super) fn save(db: &mut Connection, account: i64, discovery: &Discovery) -> Result<(), Error> {
    let tx = db.transaction()?;
    let mut paths = Vec::with_capacity(discovery.folders.len());
    for folder in &discovery.folders {
        paths.push(&folder.remote_path);
        tx.prepare_cached(
            "INSERT INTO folders(account_id, remote_path, delimiter, role, uidvalidity, uidnext)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(account_id, remote_path) DO UPDATE SET delimiter=excluded.delimiter,
               role=excluded.role, uidvalidity=excluded.uidvalidity, uidnext=excluded.uidnext",
        )?
        .execute(params![
            account,
            folder.remote_path,
            folder.delimiter,
            folder.role,
            folder.uids.map(|u| u.validity),
            folder.uids.map(|u| u.next)
        ])?;
    }
    tx.execute(
        "DELETE FROM folders WHERE account_id=?1 AND remote_path NOT IN (SELECT value FROM json_each(?2))",
        params![account, serde_json::to_string(&paths)?],
    )?;
    tx.execute(
        "INSERT INTO inbox_markers(account_id, uidvalidity, uidnext) VALUES (?1, ?2, ?3)
         ON CONFLICT(account_id) DO NOTHING",
        params![account, discovery.inbox.validity, discovery.inbox.next],
    )?;
    tx.commit()?;
    Ok(())
}

#[derive(Serialize)]
pub(crate) struct FolderView {
    id: i64,
    path: String,
    delimiter: Option<String>,
    role: Option<String>,
    hidden: bool,
    selectable: bool,
}

// Un seul dossier porte le rôle des archives : celui qui les reçoit.
pub(super) fn folders(db: &Connection, account: i64) -> rusqlite::Result<Vec<FolderView>> {
    db.prepare(
        "SELECT f.id, f.remote_path, f.delimiter,
                CASE WHEN f.id=a.folder_id THEN 'archive' WHEN f.role='archive' THEN NULL
                     ELSE f.role END,
                f.hidden, f.uidvalidity IS NOT NULL
         FROM folders f JOIN archive_folders a ON a.account_id=f.account_id
         WHERE f.account_id=?1 ORDER BY f.remote_path",
    )?
    .query_map([account], |row| {
        Ok(FolderView {
            id: row.get(0)?,
            path: display_path(row.get_ref(1)?.as_str()?),
            delimiter: row.get(2)?,
            role: row.get(3)?,
            hidden: row.get(4)?,
            selectable: row.get(5)?,
        })
    })?
    .collect()
}

/// Retient le dossier personnel qui recevra les archives du compte.
/// Rend faux si ce dossier n’est pas un dossier personnel ouvrable de ce compte.
pub(super) fn choose_archive(db: &Connection, account: i64, folder: i64) -> rusqlite::Result<bool> {
    let chosen = db.execute(
        "INSERT INTO archive_choices(account_id, folder_id)
         SELECT account_id, id FROM folders
         WHERE account_id=?1 AND id=?2 AND role IS NULL AND uidvalidity IS NOT NULL
         ON CONFLICT(account_id) DO UPDATE SET folder_id=excluded.folder_id",
        params![account, folder],
    )?;
    Ok(chosen > 0)
}
