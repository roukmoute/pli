use rusqlite::{Connection, OptionalExtension, Row, params};

use super::{Counts, Coverage, Cursor, Page, Scope, Sender, Thread};

// Dans la Corbeille ou les Indésirables, un fil est sans origine quand une de ses
// lettres n’a pas de dossier où la restaurer.
pub(super) const SELECT_PAGE: &str = "SELECT p.item,m.subject,m.sender_name,m.sender_address,m.snippet,p.received_at,
        p.message_count,p.unread,p.starred,p.attached,p.answered,p.forwarded,
        EXISTS(SELECT 1 FROM messages x JOIN folders g ON g.id=x.folder_id AND g.role IN ('trash','junk')
               WHERE x.account_id=p.account_id AND x.thread_id=p.item AND 'dossier/' || x.folder_id=p.scope
                 AND x.origin_folder IS NULL AND NOT x.moving)
    FROM list_threads p JOIN messages m
      ON (m.account_id,m.folder_id,m.uidvalidity,m.uid)=(p.account_id,p.folder_id,p.uidvalidity,p.uid)
    WHERE p.account_id=?1 AND p.scope=?2";

pub(crate) fn thread(row: &Row<'_>) -> rusqlite::Result<Thread> {
    Ok(Thread {
        id: row.get(0)?,
        objet: row.get(1)?,
        expediteur: Sender {
            nom: row.get(2)?,
            adresse: row.get(3)?,
        },
        extrait: row.get(4)?,
        date: row.get(5)?,
        nombre_messages: row.get(6)?,
        non_lu: row.get(7)?,
        etoile: row.get(8)?,
        piece_jointe: row.get(9)?,
        repondu: row.get(10)?,
        transfere: row.get(11)?,
        sans_origine: row.get(12)?,
    })
}

fn threads(
    db: &Connection,
    account: i64,
    scope: &str,
    size: usize,
    after: Option<&Cursor>,
) -> rusqlite::Result<(Vec<Thread>, Option<String>)> {
    let boundary = if after.is_some() {
        " AND (p.received_at,p.item)<(?4,?5)"
    } else {
        ""
    };
    let sql = format!("{SELECT_PAGE}{boundary} ORDER BY p.received_at DESC,p.item DESC LIMIT ?3");
    let mut statement = db.prepare_cached(&sql)?;
    let limit = (size + 1) as i64;
    let rows = match after {
        Some(c) => statement.query(params![account, scope, limit, c.date, c.fil])?,
        None => statement.query(params![account, scope, limit])?,
    };
    let mut result = rows.mapped(thread).collect::<rusqlite::Result<Vec<_>>>()?;
    let next = if result.len() > size {
        result.pop();
        let last = &result[size - 1];
        let cursor = Cursor {
            compte: account,
            portee: scope.to_owned(),
            date: last.date,
            fil: last.id.clone(),
        };
        Some(hex::encode(
            serde_json::to_vec(&cursor).expect("curseur de champs JSON scalaires"),
        ))
    } else {
        None
    };
    Ok((result, next))
}

fn coverage(db: &Connection, account: i64, scope: &Scope) -> rusqlite::Result<Option<Coverage>> {
    let (selector, folder) = match scope {
        Scope::View(_) => ("f.role='inbox'", None),
        Scope::Folder(id) => ("f.id=?2", Some(*id)),
    };
    let sql = format!(
        "SELECT f.uidnext, i.expected, i.indexed FROM folders f
        LEFT JOIN folder_imports i ON i.folder_id=f.id AND i.uidvalidity=f.uidvalidity
        WHERE f.account_id=?1 AND {selector}"
    );
    let mut statement = db.prepare_cached(&sql)?;
    let progress = |r: &Row<'_>| -> rusqlite::Result<(Option<i64>, Option<i64>)> {
        let uidnext: Option<i64> = r.get(0)?;
        Ok((
            r.get::<_, Option<i64>>(1)?
                .or_else(|| (uidnext == Some(1)).then_some(0)),
            r.get(2)?,
        ))
    };
    let row = match folder {
        Some(id) => statement
            .query_row(params![account, id], progress)
            .optional()?,
        None => statement.query_row([account], progress).optional()?,
    };
    if row.is_none() && folder.is_some() {
        return Ok(None);
    }
    let (attendus, indexed) = row.unwrap_or_default();
    let indexes = indexed.unwrap_or_default();
    let proportion_indexee = attendus.map(|n| {
        if n == 0 {
            1.0
        } else {
            indexes as f64 / n as f64
        }
    });
    Ok(Some(Coverage {
        import_en_cours: attendus != Some(indexes),
        indexes,
        attendus,
        proportion_indexee,
    }))
}

pub(super) fn read(
    db: &mut Connection,
    account: i64,
    scope: Scope,
    size: usize,
    after: Option<&Cursor>,
) -> rusqlite::Result<Option<Page>> {
    // Une seule photographie SQLite pour les lignes, les compteurs et la révision.
    let tx = db.transaction()?;
    let Some(couverture) = coverage(&tx, account, &scope)? else {
        return Ok(None);
    };
    let (rows, curseur_suivant) = threads(&tx, account, &scope.key(), size, after)?;
    let (nouveaux, deja_vus) = rows.into_iter().partition(|r| r.non_lu);
    let revision = tx.query_row(
        "SELECT revision FROM account_revisions WHERE account_id=?1",
        [account],
        |r| r.get(0),
    )?;
    let (compteurs, expediteurs_en_attente) = tx.prepare_cached(
        "SELECT coalesce(max(total) FILTER(WHERE scope='reception'),0),
                coalesce(max(total) FILTER(WHERE scope='flux'),0),
                coalesce(max(total) FILTER(WHERE scope='justificatifs'),0),
                coalesce(max(total) FILTER(WHERE scope='tri'),0)
         FROM list_counts WHERE account_id=?1 AND scope IN ('reception','flux','justificatifs','tri')"
    )?.query_row([account], |row| {
        let counts = Counts { reception: row.get(0)?, flux: row.get(1)?, justificatifs: row.get(2)? };
        Ok((counts, row.get(3)?))
    })?;
    let total = tx
        .prepare_cached("SELECT total FROM list_counts WHERE (account_id,scope)=(?1,?2)")?
        .query_row(params![account, scope.key()], |row| row.get(0))
        .optional()?
        .unwrap_or_default();
    let (vue, dossier) = match scope {
        Scope::View(v) => (Some(v), None),
        Scope::Folder(id) => (None, Some(id)),
    };
    tx.commit()?;
    Ok(Some(Page {
        compte: account,
        vue,
        dossier,
        nouveaux,
        deja_vus,
        curseur_suivant,
        total,
        revision,
        couverture,
        compteurs,
        expediteurs_en_attente,
    }))
}
