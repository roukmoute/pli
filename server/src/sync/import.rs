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

async fn import_range(
    db: &Arc<Database>,
    session: &mut ImapSession,
    mailbox: &Mailbox,
    start: u32,
    end: u32,
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
    let pending =
        spawn_blocking(move || messages::missing(&*reader.reader()?, &scope, &remote)).await??;
    for ids in pending.chunks(BATCH_SIZE) {
        let headers = fetch(session, ids, mailbox.local_test).await?;
        let writer = db.clone();
        let scope = mailbox.clone();
        spawn_blocking(move || messages::save_batch(&mut *writer.writer()?, &scope, &headers))
            .await??;
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
        import_range(db, &mut session, mailbox, 1, mailbox.next).await?;
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
        import_range(&context.db, &mut session, inbox, start, end).await?;
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
    use super::uid_search;

    #[test]
    fn only_requests_a_nonempty_inclusive_imap_range() {
        assert_eq!(uid_search(52, 52), None);
        assert_eq!(uid_search(53, 52), None);
        assert_eq!(uid_search(52, 53).as_deref(), Some("UID SEARCH UID 52:52"));
    }
}
