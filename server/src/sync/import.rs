use std::{sync::Arc, time::Duration};

use async_imap::imap_proto::{MailboxDatum, Response};
use tokio::{
    task::spawn_blocking,
    time::{sleep, timeout},
};

use super::{
    Context, discovery, header,
    imap::{COMMAND_TIMEOUT, command},
    messages::{self, Mailbox},
};
use crate::{Error, admin::ImapSession, db::Database};

const BATCH_SIZE: usize = 50;
const POLL_INTERVAL: Duration = Duration::from_secs(5);

async fn examine(session: &mut ImapSession, mailbox: &Mailbox) -> Result<Option<u32>, Error> {
    let selected = timeout(COMMAND_TIMEOUT, session.examine(&mailbox.path)).await??;
    if selected.uid_validity != Some(mailbox.validity) {
        return Err("La génération IMAP du dossier a changé.".into());
    }
    Ok(selected.uid_next)
}

async fn fetch(
    session: &mut ImapSession,
    ids: &[u32],
    local_test: bool,
) -> Result<Vec<header::Header>, Error> {
    let set = ids.iter().map(u32::to_string).collect::<Vec<_>>().join(",");
    let query = format!("UID FETCH {set} {}", header::ITEMS);
    let mut messages = Vec::with_capacity(ids.len());
    command(session, &query, |response| {
        if let Response::Fetch(_, attrs) = response
            && let Some(header) = header::parse(attrs)?
            && ids.contains(&header.uid)
        {
            messages.push(header);
        }
        Ok(())
    })
    .await?;
    super::preview::load(session, &mut messages, local_test).await?;
    messages.sort_unstable_by_key(|m| std::cmp::Reverse(m.uid));
    Ok(messages)
}

// IMAP interprète une plage inversée dans les deux sens et inclut ses deux bornes.
fn uid_search(start: u32, end: u32) -> Option<String> {
    (start < end).then(|| format!("UID SEARCH UID {start}:{}", end - 1))
}

fn plan_history(
    db: &Database,
    mailbox: &Mailbox,
    end: u32,
    mut remote: Vec<u32>,
) -> Result<Vec<u32>, Error> {
    remote.sort_unstable();
    remote.dedup();
    let mut writer = db.writer()?;
    let tx = writer.transaction()?;
    let pending = messages::missing(&tx, mailbox, &remote)?;
    let indexed = remote.len() - pending.len();
    let changed = tx.execute(
        "INSERT INTO folder_imports(folder_id,uidvalidity,until_uid,expected,indexed)
         SELECT id,uidvalidity,?3,?4,?5 FROM folders WHERE id=?1 AND uidvalidity=?2
         ON CONFLICT(folder_id) DO UPDATE SET uidvalidity=excluded.uidvalidity,
         until_uid=excluded.until_uid,expected=excluded.expected,indexed=excluded.indexed",
        rusqlite::params![
            mailbox.folder,
            mailbox.validity,
            end,
            remote.len() as i64,
            indexed as i64
        ],
    )?;
    if changed == 0 {
        return Err("La génération IMAP du dossier a changé.".into());
    }
    tx.commit()?;
    Ok(pending)
}

async fn import_range(
    db: &Arc<Database>,
    session: &mut ImapSession,
    mailbox: &Mailbox,
    start: u32,
    end: u32,
    historique: bool,
) -> Result<(), Error> {
    let Some(query) = uid_search(start, end) else {
        return Ok(());
    };
    // SEARCH n'énumère que les UID existants, même dans un dossier très clairsemé.
    let mut remote = Vec::new();
    command(session, &query, |response| {
        if let Response::MailboxData(MailboxDatum::Search(found)) = response {
            remote.extend(found.iter().copied().filter(|id| (start..end).contains(id)));
        }
        Ok(())
    })
    .await?;
    let reader = db.clone();
    let scope = mailbox.clone();
    let pending = spawn_blocking(move || -> Result<_, Error> {
        if historique {
            plan_history(&reader, &scope, end, remote)
        } else {
            messages::missing(&*reader.reader()?, &scope, &remote)
        }
    })
    .await??;
    for ids in pending.chunks(BATCH_SIZE) {
        let headers = fetch(session, ids, mailbox.local_test).await?;
        let writer = db.clone();
        let scope = mailbox.clone();
        spawn_blocking(move || messages::save_batch(&mut *writer.writer()?, &scope, &headers))
            .await??;
    }
    if historique {
        // Tous les FETCH ont abouti. Un UID expurgé après SEARCH sort du total.
        let db = db.clone();
        let scope = mailbox.clone();
        spawn_blocking(move || -> Result<_, Error> {
            db.writer()?.execute(
                "UPDATE folder_imports SET expected=indexed WHERE folder_id=?1 AND uidvalidity=?2 AND until_uid=?3",
                rusqlite::params![scope.folder,scope.validity,end],
            )?;
            Ok(())
        }).await??;
    }
    Ok(())
}

async fn history(
    db: &Arc<Database>,
    mut session: ImapSession,
    mailboxes: &[Mailbox],
) -> Result<(), Error> {
    for mailbox in mailboxes.iter().filter(|m| m.next > 1) {
        examine(&mut session, mailbox).await?;
        import_range(db, &mut session, mailbox, 1, mailbox.next, true).await?;
    }
    let _ = timeout(Duration::from_secs(1), session.logout()).await;
    Ok(())
}

async fn arrivals(context: &Arc<Context>, inbox: &Mailbox) -> Result<(), Error> {
    let mut start = inbox.next;
    sleep(POLL_INTERVAL).await;
    let mut session = context
        .reconnect(inbox.account)
        .await
        .map_err(|_| "Connexion IMAP à reprendre.")?;
    loop {
        let end = match examine(&mut session, inbox).await? {
            Some(next) => next,
            None => discovery::status(&mut session, &inbox.path)
                .await?
                .next
                .get(),
        };
        import_range(&context.db, &mut session, inbox, start, end, false).await?;
        start = end;
        sleep(POLL_INTERVAL).await;
    }
}

pub(super) async fn run(
    context: &Arc<Context>,
    session: ImapSession,
    mailboxes: &[Mailbox],
) -> Result<(), Error> {
    let inbox = mailboxes.iter().find(|m| m.inbox).ok_or("INBOX absente.")?;
    // Le sondage dispose de sa connexion, même si FETCH attend encore un littéral d'historique.
    tokio::try_join!(
        history(&context.db, session, mailboxes),
        arrivals(context, inbox)
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{messages, plan_history, uid_search};
    use crate::{
        Error,
        test_support::{BOITES_DE_TEST, TestApp},
    };

    #[test]
    fn historique_clairseme_compte_les_uid_uniques_et_refuse_une_generation_perimee()
    -> Result<(), Error> {
        let app = TestApp::new()?;
        let inbox = {
            let db = app.db.writer()?;
            db.execute_batch(BOITES_DE_TEST)?;
            db.execute_batch(
                "INSERT INTO messages(account_id,folder_id,uidvalidity,uid,thread_id,
                recipients,subject,received_at,flags,size,has_attachments,snippet)
                VALUES(1,1,1,4,'ancien','{}','Ancienne lettre',0,'[]',0,0,'')",
            )?;
            messages::mailboxes(&db, 1)?.remove(0)
        };
        assert_eq!(
            plan_history(&app.db, &inbox, 200, vec![20, 4, 4, 150, 20])?,
            [150, 20]
        );
        let etat = || -> Result<(i64, i64, i64), Error> {
            Ok(app.db.reader()?.query_row(
                "SELECT until_uid,expected,indexed FROM folder_imports WHERE folder_id=1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )?)
        };
        assert_eq!(etat()?, (200, 3, 1));
        assert!(plan_history(&app.db, &inbox, 201, vec![4])?.is_empty());
        assert_eq!(etat()?, (201, 1, 1));
        app.db
            .writer()?
            .execute("UPDATE folders SET uidvalidity=2 WHERE id=1", [])?;
        assert!(plan_history(&app.db, &inbox, 200, vec![20, 4]).is_err());
        assert!(etat().is_err());
        Ok(())
    }

    #[test]
    fn only_requests_a_nonempty_inclusive_imap_range() {
        assert_eq!(uid_search(52, 52), None);
        assert_eq!(uid_search(53, 52), None);
        assert_eq!(uid_search(52, 53).as_deref(), Some("UID SEARCH UID 52:52"));
    }
}
