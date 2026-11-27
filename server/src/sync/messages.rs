use rusqlite::{Connection, Transaction, params};
use serde::Serialize;

use super::header::Header;
use crate::Error;

#[derive(Clone)]
pub(super) struct Mailbox {
    pub account: i64,
    pub folder: i64,
    pub path: String,
    pub validity: u32,
    pub next: u32,
    pub inbox: bool,
    pub local_test: bool,
    screen_from: Option<u32>,
}

pub(super) fn mailboxes(db: &Connection, account: i64) -> rusqlite::Result<Vec<Mailbox>> {
    db.prepare(
        "SELECT f.id, f.remote_path, f.uidvalidity, f.uidnext, f.role IS 'inbox',
                CASE WHEN f.role='inbox' AND f.uidvalidity=b.uidvalidity THEN b.uidnext END, p.local_test
         FROM folders f JOIN inbox_markers b ON b.account_id=f.account_id
         JOIN accounts a ON a.id=f.account_id JOIN server_profiles p ON p.id=a.server_profile_id
         WHERE f.account_id=?1 AND f.uidnext IS NOT NULL ORDER BY f.role IS 'inbox' DESC, f.id",
    )?
    .query_map([account], |r| {
        Ok(Mailbox {
            account,
            folder: r.get(0)?,
            path: r.get(1)?,
            validity: r.get(2)?,
            next: r.get(3)?,
            inbox: r.get(4)?,
            screen_from: r.get(5)?,
            local_test: r.get(6)?,
        })
    })?
    .collect()
}

pub(super) fn missing(
    db: &Connection,
    mailbox: &Mailbox,
    remote: &[u32],
) -> Result<Vec<u32>, Error> {
    let mut query = db.prepare_cached(
        "SELECT DISTINCT value FROM json_each(?4) WHERE NOT EXISTS (
            SELECT 1 FROM messages WHERE account_id=?1 AND folder_id=?2 AND uidvalidity=?3 AND uid=value
         ) ORDER BY value DESC",
    )?;
    let rows = query.query_map(
        params![
            mailbox.account,
            mailbox.folder,
            mailbox.validity,
            serde_json::to_string(remote)?
        ],
        |r| r.get(0),
    )?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

fn thread(tx: &Transaction<'_>, mailbox: &Mailbox, message: &Header) -> Result<String, Error> {
    let account = mailbox.account;
    let ids: Vec<_> = message
        .references
        .iter()
        .chain(&message.message_id)
        .collect();
    let Some(first) = ids.first() else {
        return Ok(format!(
            "{account}/uid/{}/{}/{}",
            mailbox.folder, mailbox.validity, message.uid
        ));
    };
    let links = serde_json::to_string(&ids)?;
    let threads: Vec<String> = tx
        .prepare_cached(
            "SELECT DISTINCT thread_id FROM thread_references WHERE account_id=?1
           AND message_id IN (SELECT value FROM json_each(?2)) ORDER BY thread_id",
        )?
        .query_map(params![account, links], |r| r.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    let mut threads = threads.into_iter();
    let thread = threads
        .next()
        .unwrap_or_else(|| format!("{account}/id/{first}"));
    // Une réponse découverte tard peut relier deux groupes déjà importés.
    for previous in threads {
        for sql in [
            "UPDATE messages SET thread_id=?3 WHERE account_id=?1 AND thread_id=?2",
            "UPDATE thread_references SET thread_id=?3 WHERE account_id=?1 AND thread_id=?2",
        ] {
            tx.prepare_cached(sql)?
                .execute(params![account, previous, thread])?;
        }
    }
    tx.prepare_cached(
        "INSERT INTO thread_references SELECT ?1, value, ?3 FROM json_each(?2) WHERE true
         ON CONFLICT(account_id, message_id) DO NOTHING",
    )?
    .execute(params![account, links, thread])?;
    Ok(thread)
}

fn insert(tx: &Transaction<'_>, mailbox: &Mailbox, m: &Header) -> Result<(), Error> {
    let address = m.sender.as_ref().map(|s| &s.address);
    if let Some(address) = address
        && mailbox.screen_from.is_some_and(|start| m.uid >= start)
    {
        tx.prepare_cached(
            "INSERT INTO sender_decisions(account_id,address) VALUES (?1,?2)
             ON CONFLICT(account_id,address) DO NOTHING",
        )?
        .execute(params![mailbox.account, address])?;
    }
    let thread = thread(tx, mailbox, m)?;
    tx.prepare_cached(
        "INSERT INTO messages(account_id,folder_id,uidvalidity,uid,message_id,sender_address,
            sender_name,recipients,subject,sent_at,received_at,flags,size,has_attachments,snippet,thread_id)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16)
         ON CONFLICT(account_id,folder_id,uidvalidity,uid) DO NOTHING",
    )?
    .execute(params![
        mailbox.account, mailbox.folder, mailbox.validity, m.uid, m.message_id, address,
        m.sender.as_ref().and_then(|s| s.name.as_ref()), serde_json::to_string(&m.recipients)?,
        m.subject, m.sent_at, m.received_at, serde_json::to_string(&m.flags)?, m.size,
        m.structure.attached, m.snippet, thread
    ])?;
    Ok(())
}

pub(super) fn save_batch(
    db: &mut Connection,
    mailbox: &Mailbox,
    messages: &[Header],
) -> Result<(), Error> {
    let tx = db.transaction()?;
    tx.query_row(
        "SELECT 1 FROM folders WHERE account_id=?1 AND id=?2 AND uidvalidity=?3",
        params![mailbox.account, mailbox.folder, mailbox.validity],
        |_| Ok(()),
    )?;
    for message in messages {
        insert(&tx, mailbox, message)?;
    }
    tx.commit()?;
    Ok(())
}

#[derive(Serialize)]
pub(crate) struct PendingSender {
    address: String,
    name: Option<String>,
    last_subject: String,
    snippet: String,
    message_count: i64,
    revision: i64,
}

pub(super) fn pending(db: &Connection, account: i64) -> rusqlite::Result<Vec<PendingSender>> {
    db.prepare(
        "WITH pending AS (
            SELECT sender_address, sender_name, subject, snippet, uid,
                   count(*) OVER (PARTITION BY sender_address) AS total,
                   row_number() OVER (PARTITION BY sender_address ORDER BY uid DESC) AS position
            FROM routed_messages WHERE account_id=?1 AND screening_state='en_attente'
         )
         SELECT p.sender_address,p.sender_name,p.subject,p.snippet,p.total,d.revision
         FROM pending p JOIN sender_decisions d ON d.account_id=?1 AND d.address=p.sender_address
         WHERE p.position=1 ORDER BY p.uid DESC",
    )?
    .query_map([account], |r| {
        Ok(PendingSender {
            address: r.get(0)?,
            name: r.get(1)?,
            last_subject: r.get(2)?,
            snippet: r.get(3)?,
            message_count: r.get(4)?,
            revision: r.get(5)?,
        })
    })?
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{sync::header, test_support::TestApp};
    use async_imap::imap_proto::Response;
    use serde_json::json;

    fn message(uid: u32, id: &str, reply: &str, references: &str) -> Header {
        let fields = format!("References: {references}\r\n\r\n");
        let raw = format!(
            "* 1 FETCH (UID {uid} ENVELOPE (NIL \"Même objet\" ((\"=?UTF-8?Q?Th=C3=A9o?=\" NIL \"a\" \"RECETTE.TEST\")) NIL NIL ((NIL NIL \"b\" \"recette.test\")) NIL NIL {reply} {id}) FLAGS (\\Seen) INTERNALDATE \" 1-Jan-2025 00:00:00 +0000\" RFC822.SIZE 42 BODYSTRUCTURE (\"TEXT\" \"PLAIN\" NIL NIL NIL \"7BIT\" 4 1) BODY[HEADER.FIELDS (REFERENCES)] {{{}}}\r\n{fields})\r\n",
            fields.len()
        );
        let (_, Response::Fetch(_, attrs)) = Response::parse(raw.as_bytes()).unwrap() else {
            panic!("FETCH attendu")
        };
        header::parse(&attrs).unwrap().unwrap()
    }

    fn setup(db: &Connection) {
        db.execute_batch("INSERT INTO server_profiles VALUES(1,'Banc','localhost',993,'implicit','localhost',465,'implicit','[]',1,1);
            INSERT INTO users VALUES(1),(2);
            INSERT INTO accounts VALUES(1,1,'alice@recette.test',1,x''),(2,2,'bob@recette.test',1,x'');
            INSERT INTO folders(id,account_id,remote_path,role,uidvalidity,uidnext)
                VALUES(1,1,'INBOX','inbox',73,10),(2,1,'Dossier',NULL,73,10),(3,2,'INBOX','inbox',73,10);
            INSERT INTO inbox_markers VALUES(1,73,5),(2,73,5);").unwrap();
    }

    #[test]
    fn routes_by_account_generation_folder_and_boundary_and_preserves_existing_choices()
    -> Result<(), Error> {
        let app = TestApp::new()?;
        let mut db = app.db.writer()?;
        setup(&db);
        let mailboxes = mailboxes(&db, 1)?;
        let (inbox, folder) = (&mailboxes[0], &mailboxes[1]);
        for mailbox in [inbox, folder] {
            let mut letters = [
                message(4, "NIL", "NIL", ""),
                message(5, "NIL", "NIL", ""),
                message(9, "NIL", "NIL", ""),
            ];
            letters[2].subject = "Dernier".into();
            letters[2].snippet = "Extrait".into();
            for _ in 0..2 {
                save_batch(&mut db, mailbox, &letters)?;
            }
        }
        assert_eq!(missing(&db, inbox, &[4, 7, 9, 42, 2, 2])?, [42, 7, 2]);
        assert_eq!(
            serde_json::to_value(pending(&db, 1)?)?,
            json!([{
                "address":"a@recette.test","name":"Théo","last_subject":"Dernier","snippet":"Extrait","message_count":2,"revision":1
            }])
        );
        assert!(pending(&db, 2)?.is_empty());
        for (state, destination) in [("accepte", Some("flux")), ("refuse", None)] {
            db.execute(
                "UPDATE sender_decisions SET state=?1,destination='flux',revision=7",
                [state],
            )?;
            save_batch(&mut db, inbox, &[message(10, "NIL", "NIL", "")])?;
            let mut q = db.prepare(
                "SELECT destination FROM routed_messages WHERE folder_id=1 ORDER BY uid",
            )?;
            let rows = q
                .query_map([], |r| r.get::<_, Option<String>>(0))?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            assert_eq!(
                rows.iter().map(Option::as_deref).collect::<Vec<_>>(),
                [Some("reception"), destination, destination, destination]
            );
            assert!(pending(&db, 1)?.is_empty());
        }
        assert_eq!(db.query_row("SELECT count(*) FROM routed_messages WHERE folder_id=2 AND destination IS NULL AND screening_state IS NULL",[],|r| r.get::<_,u32>(0))?,3);
        assert_eq!(
            db.query_row("SELECT revision FROM sender_decisions", [], |r| r
                .get::<_, u32>(0))?,
            7
        );
        db.execute("UPDATE folders SET uidvalidity=74 WHERE id=1", [])?;
        assert!(save_batch(&mut db, inbox, &[message(8, "NIL", "NIL", "")]).is_err());
        assert_eq!(
            db.query_row(
                "SELECT count(*) FROM routed_messages WHERE folder_id=1",
                [],
                |r| r.get::<_, u32>(0)
            )?,
            0
        );
        Ok(())
    }

    #[test]
    fn a_failed_batch_can_be_replayed_with_its_threads_and_decisions() -> Result<(), Error> {
        let app = TestApp::new()?;
        let mut db = app.db.writer()?;
        setup(&db);
        let inbox = mailboxes(&db, 1)?.remove(0);
        let letters = [
            message(6, "\"<later>\"", "\"<parent>\"", ""),
            message(5, "\"<parent>\"", "NIL", ""),
        ];
        db.execute_batch(
            "CREATE TRIGGER interrompre BEFORE INSERT ON messages WHEN NEW.uid=5
            BEGIN SELECT RAISE(ABORT, 'lot interrompu'); END;",
        )?;
        assert!(save_batch(&mut db, &inbox, &letters).is_err());
        for table in ["messages", "sender_decisions", "thread_references"] {
            assert_eq!(
                db.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r
                    .get::<_, u32>(0))?,
                0
            );
        }
        db.execute_batch("DROP TRIGGER interrompre")?;
        save_batch(&mut db, &inbox, &letters)?;
        assert_eq!(pending(&db, 1)?[0].message_count, 2);
        assert!(missing(&db, &inbox, &[5, 6])?.is_empty());
        Ok(())
    }

    #[test]
    fn merges_references_seen_out_of_order_without_collapsing_copies_or_equal_subjects()
    -> Result<(), Error> {
        let app = TestApp::new()?;
        let mut db = app.db.writer()?;
        setup(&db);
        for account in [1, 2] {
            let inbox = mailboxes(&db, account)?.remove(0);
            for (uid, id, reply, refs) in [
                (9, "\"<child-b>\"", "\"<b>\"", ""),
                (8, "\"<child-a>\"", "NIL", "<a>"),
                (7, "\"<bridge>\"", "\"<b>\"", "<a>"),
                (4, "\"<a>\"", "NIL", ""),
                (3, "\"<a>\"", "NIL", ""),
                (2, "NIL", "NIL", ""),
                (1, "NIL", "NIL", ""),
            ] {
                save_batch(&mut db, &inbox, &[message(uid, id, reply, refs)])?;
            }
            let mut q=db.prepare("SELECT count(*) FROM messages WHERE account_id=?1 GROUP BY thread_id ORDER BY count(*)")?;
            let counts = q
                .query_map([account], |r| r.get::<_, u32>(0))?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            assert_eq!(counts, [1, 1, 5]);
        }
        assert_eq!(
            db.query_row("SELECT count(DISTINCT thread_id) FROM messages", [], |r| {
                r.get::<_, u32>(0)
            })?,
            6
        );
        Ok(())
    }
}
