use std::sync::Arc;

use async_imap::imap_proto::{AttributeValue, Response};
use tokio::time::timeout;

use super::{
    Context, Synchronizer,
    imap::{COMMAND_TIMEOUT, command},
};
use crate::{Error, admin::ImapSession, lecture::Cle};

#[derive(Clone)]
pub(crate) struct Lecteur {
    context: Arc<Context>,
}

impl Synchronizer {
    pub(crate) fn lecteur(&self) -> Lecteur {
        Lecteur {
            context: self.context.clone(),
        }
    }
}

impl Lecteur {
    async fn session(
        &self,
        compte: i64,
        cle: Cle,
        chemin: &str,
        ecriture: bool,
    ) -> Result<ImapSession, Error> {
        let mut session = self
            .context
            .reconnect(compte)
            .await
            .map_err(|_| "Messagerie indisponible.")?;
        let dossier = if ecriture {
            timeout(COMMAND_TIMEOUT, session.select(chemin)).await??
        } else {
            timeout(COMMAND_TIMEOUT, session.examine(chemin)).await??
        };
        if dossier.uid_validity != Some(cle.generation) {
            return Err("La génération IMAP du dossier a changé.".into());
        }
        Ok(session)
    }

    pub(crate) async fn contenu(
        &self,
        compte: i64,
        cle: Cle,
        chemin: &str,
    ) -> Result<Option<Vec<u8>>, Error> {
        let mut session = self.session(compte, cle, chemin, false).await?;
        let mut body = None;
        command(
            &mut session,
            &format!("UID FETCH {} (UID BODY.PEEK[])", cle.uid),
            |reply| {
                if let Some(bytes) = attributs(reply, cle.uid).and_then(|attrs| {
                    attrs.iter().find_map(|attr| match attr {
                        AttributeValue::BodySection {
                            section: None,
                            data: Some(bytes),
                            ..
                        } => Some(bytes),
                        _ => None,
                    })
                }) {
                    body = Some(bytes.to_vec());
                }
                Ok(())
            },
        )
        .await?;
        Ok(body)
    }

    pub(crate) async fn ouvrir(&self, compte: i64, cle: Cle, chemin: &str) -> Result<bool, Error> {
        let mut session = self.session(compte, cle, chemin, true).await?;
        let mut lu = false;
        let commande = format!("UID STORE {} +FLAGS (\\Seen)", cle.uid);
        command(&mut session, &commande, |reply| {
            if let Some(attrs) = attributs(reply, cle.uid) {
                lu |= attrs.iter().any(indique_lu);
            }
            Ok(())
        })
        .await?;
        Ok(lu)
    }
}

fn indique_lu(attr: &AttributeValue<'_>) -> bool {
    matches!(attr, AttributeValue::Flags(flags) if flags.iter().any(|f| f.eq_ignore_ascii_case("\\Seen")))
}

fn attributs<'a>(reply: &'a Response<'_>, uid: u32) -> Option<&'a [AttributeValue<'a>]> {
    let Response::Fetch(_, attrs) = reply else {
        return None;
    };
    attrs
        .iter()
        .any(|a| matches!(a,AttributeValue::Uid(id) if *id==uid))
        .then_some(attrs)
}
