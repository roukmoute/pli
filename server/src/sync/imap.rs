use std::time::Duration;

use async_imap::imap_proto::{Response, Status};
use tokio::time::timeout;

use crate::{Error, admin::ImapSession};

pub(super) const COMMAND_TIMEOUT: Duration = Duration::from_secs(15);

// Les flux LIST/FETCH de la bibliothèque se terminent aussi sur NO ou sur EOF.
// Seule la réponse OK étiquetée permet de publier le lot ou d'avancer le repère de lecture.
pub(super) async fn command(
    session: &mut ImapSession,
    command: &str,
    mut collect: impl FnMut(&Response<'_>) -> Result<(), Error>,
) -> Result<(), Error> {
    timeout(COMMAND_TIMEOUT, async {
        let tag = session.run_command(command).await?;
        while let Some(reply) = session.read_response().await? {
            match reply.parsed() {
                Response::Done {
                    tag: end, status, ..
                } if *end == tag => {
                    return match status {
                        Status::Ok => Ok(()),
                        _ => Err("Commande IMAP refusée.".into()),
                    };
                }
                response => collect(response)?,
            }
        }
        Err("Commande IMAP interrompue.".into())
    })
    .await?
}
