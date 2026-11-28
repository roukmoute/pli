mod deplacement;
mod operations;

use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
};

use async_imap::imap_proto::{AttributeValue, Response};
use tokio::{
    sync::{Mutex, Notify, OwnedMutexGuard},
    time::timeout,
};

use super::{
    Context, Synchronizer,
    imap::{COMMAND_TIMEOUT, Refus, command},
};
use crate::{Error, admin::ImapSession, lecture::Cle};

#[derive(Clone)]
pub(crate) struct Lecteur {
    context: Arc<Context>,
    ecritures: Arc<Mutex<HashMap<i64, Arc<Mutex<()>>>>>,
    files: Arc<std::sync::Mutex<HashMap<i64, Arc<Notify>>>>,
}

impl Synchronizer {
    /// Les copies du lecteur partagent ses verrous d’écriture et ses files d’opérations.
    pub(crate) fn lecteur(&self) -> Lecteur {
        let lecteur = Lecteur {
            context: self.context.clone(),
            ecritures: Default::default(),
            files: Default::default(),
        };
        lecteur.reprendre();
        lecteur
    }
}

impl Lecteur {
    pub(crate) async fn ecriture(&self, compte: i64) -> OwnedMutexGuard<()> {
        let verrou = self
            .ecritures
            .lock()
            .await
            .entry(compte)
            .or_default()
            .clone();
        verrou.lock_owned().await
    }

    async fn session(
        &self,
        compte: i64,
        generation: u32,
        chemin: &str,
        ecriture: bool,
    ) -> Result<ImapSession, Error> {
        let mut session = self
            .context
            .reconnect(compte)
            .await
            .map_err(|_| "Messagerie indisponible.")?;
        let choix = if ecriture {
            timeout(COMMAND_TIMEOUT, session.select(chemin)).await?
        } else {
            timeout(COMMAND_TIMEOUT, session.examine(chemin)).await?
        };
        let dossier = match choix {
            Err(async_imap::error::Error::No(_) | async_imap::error::Error::Bad(_)) => {
                return Err(Refus.into());
            }
            autre => autre?,
        };
        // Une autre génération désigne d’autres lettres sous les mêmes UID.
        if dossier.uid_validity != Some(generation) {
            return Err(Refus.into());
        }
        Ok(session)
    }

    pub(crate) async fn contenu(
        &self,
        compte: i64,
        cle: Cle,
        chemin: &str,
    ) -> Result<Option<Vec<u8>>, Error> {
        let mut session = self.session(compte, cle.generation, chemin, false).await?;
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

    pub(crate) async fn marquer(
        &self,
        compte: i64,
        generation: u32,
        chemin: &str,
        uids: &[u32],
        drapeau: &str,
        valeur: bool,
    ) -> Result<BTreeMap<u32, bool>, Error> {
        let mut session = self.session(compte, generation, chemin, true).await?;
        let ids = ensemble(uids);
        let signe = if valeur { "+" } else { "-" };
        let commande = format!("UID STORE {ids} {signe}FLAGS ({drapeau})");
        let mut etats = BTreeMap::new();
        command(&mut session, &commande, |reponse| {
            if let Response::Fetch(_, attrs) = reponse
                && let Some(uid) = attrs.iter().find_map(|a| match a {
                    AttributeValue::Uid(uid) => Some(*uid),
                    _ => None,
                })
                && let Some(flags) = attrs.iter().find_map(|a| match a {
                    AttributeValue::Flags(flags) => Some(flags),
                    _ => None,
                })
            {
                etats.insert(
                    uid,
                    flags.iter().any(|f| f.eq_ignore_ascii_case(drapeau)) == valeur,
                );
            }
            Ok(())
        })
        .await?;
        Ok(etats)
    }
}

// Ensemble d’UID d’une commande IMAP.
fn ensemble(uids: &[u32]) -> String {
    uids.iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join(",")
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
