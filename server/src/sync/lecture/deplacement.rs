//! Déplacement de lettres d’un dossier à un autre chez l’hébergeur.

use std::collections::BTreeMap;

use async_imap::{
    imap_proto::{MailboxDatum, Response, ResponseCode, rfc4315::UidSetMember},
    types::Capability,
};
use tokio::time::timeout;

use super::{Lecteur, ensemble};
use crate::{
    Error,
    admin::ImapSession,
    operations::{Depot, Sort},
    sync::imap::{COMMAND_TIMEOUT, Refus, command},
};

// Un lot compte cent lettres : une réponse plus longue ne le décrit plus.
const CORRESPONDANCES: usize = 1000;

/// Dossier de départ ouvert, avec les lettres du lot qui s’y trouvent encore.
pub(crate) struct Depart {
    session: ImapSession,
    pub presentes: Vec<u32>,
}

async fn presentes(session: &mut ImapSession, uids: &[u32]) -> Result<Vec<u32>, Error> {
    let mut trouvees = Vec::new();
    let recherche = format!("UID SEARCH UID {}", ensemble(uids));
    command(session, &recherche, |reponse| {
        if let Response::MailboxData(MailboxDatum::Search(ids)) = reponse {
            trouvees.extend(uids.iter().copied().filter(|uid| ids.contains(uid)));
        }
        Ok(())
    })
    .await?;
    Ok(trouvees)
}

// Chaîne entre guillemets de RFC 3501. Un saut de ligne ouvrirait une autre commande.
fn boite(chemin: &str) -> Result<String, Refus> {
    if chemin.contains(['\r', '\n']) {
        return Err(Refus);
    }
    Ok(format!(
        "\"{}\"",
        chemin.replace('\\', "\\\\").replace('"', "\\\"")
    ))
}

fn suite(membres: &[UidSetMember]) -> Vec<u32> {
    membres
        .iter()
        .flat_map(|membre| match membre {
            UidSetMember::Uid(uid) => *uid..=*uid,
            UidSetMember::UidRange(plage) => plage.clone(),
        })
        .take(CORRESPONDANCES)
        .collect()
}

/// Nouveaux UID annoncés par le serveur (RFC 4315) et génération du dossier d’arrivée.
#[derive(Default)]
struct Copie {
    generation: Option<u32>,
    nouveaux: BTreeMap<u32, u32>,
}

impl Copie {
    // Les deux listes se répondent terme à terme, dans l’ordre de la copie.
    // Des longueurs différentes rattacheraient une lettre à l’UID d’une autre.
    fn relever(&mut self, reponse: &Response<'_>) {
        let (Response::Data { outcome, .. } | Response::Done { outcome, .. }) = reponse else {
            return;
        };
        let Some(ResponseCode::CopyUid(generation, depart, arrivee)) = &outcome.code else {
            return;
        };
        let (depart, arrivee) = (suite(depart), suite(arrivee));
        if depart.len() != arrivee.len() {
            return;
        }
        self.generation = Some(*generation);
        self.nouveaux.extend(depart.into_iter().zip(arrivee));
    }
}

// Marque puis efface les seules lettres du lot (RFC 4315).
async fn retirer(session: &mut ImapSession, lettres: &str) -> Result<(), Error> {
    let marque = format!("UID STORE {lettres} +FLAGS.SILENT (\\Deleted)");
    command(session, &marque, |_| Ok(())).await?;
    command(session, &format!("UID EXPUNGE {lettres}"), |_| Ok(())).await
}

// Sans MOVE : copie, puis retire les lettres copiées.
async fn copier_puis_retirer(
    session: &mut ImapSession,
    lettres: &str,
    boite: &str,
    copie: &mut Copie,
) -> Result<(), Error> {
    command(session, &format!("UID COPY {lettres} {boite}"), |reponse| {
        copie.relever(reponse);
        Ok(())
    })
    .await?;
    retirer(session, lettres).await
}

impl Lecteur {
    /// Ouvre le dossier de départ et relève les lettres du lot qui s’y trouvent encore.
    pub(crate) async fn partir(
        &self,
        compte: i64,
        generation: u32,
        chemin: &str,
        uids: &[u32],
    ) -> Result<Depart, Error> {
        let mut session = self.session(compte, generation, chemin, true).await?;
        let presentes = presentes(&mut session, uids).await?;
        Ok(Depart { session, presentes })
    }
}

// Envoie les commandes qui déplacent les lettres vers `arrivee`, ou qui les effacent
// faute d’arrivée, et relève les UID annoncés. Sans effacement ciblé, d’autres lettres
// marquées supprimées partiraient aussi : la demande est alors refusée.
// Après un refus, la recherche qui suit tranche lettre par lettre.
async fn envoyer(
    session: &mut ImapSession,
    uids: &[u32],
    arrivee: Option<&str>,
    copie: &mut Copie,
) -> Result<(), Error> {
    let boite = arrivee.map(boite).transpose()?;
    let lettres = ensemble(uids);
    let capacites = timeout(COMMAND_TIMEOUT, session.capabilities()).await??;
    let sait = |nom: &str| {
        capacites
            .iter()
            .any(|c| matches!(c, Capability::Atom(atome) if atome.eq_ignore_ascii_case(nom)))
    };
    let envoi = match boite {
        Some(boite) if sait("MOVE") => {
            let commande = format!("UID MOVE {lettres} {boite}");
            command(session, &commande, |reponse| {
                copie.relever(reponse);
                Ok(())
            })
            .await
        }
        Some(boite) if sait("UIDPLUS") => {
            copier_puis_retirer(session, &lettres, &boite, copie).await
        }
        None if sait("UIDPLUS") => retirer(session, &lettres).await,
        _ => Err(Refus.into()),
    };
    match envoi {
        Err(panne) if !panne.is::<Refus>() => Err(panne),
        _ => Ok(()),
    }
}

impl Depart {
    /// Déplace les lettres présentes vers `arrivee`, ou les efface faute d’arrivée,
    /// puis constate lesquelles sont parties.
    pub(crate) async fn transferer(mut self, arrivee: Option<&str>) -> Result<Depot, Error> {
        let mut copie = Copie::default();
        let mut restees = Vec::new();
        // Sans lettre présente, aucune commande ne part.
        if !self.presentes.is_empty() {
            envoyer(&mut self.session, &self.presentes, arrivee, &mut copie).await?;
            restees = presentes(&mut self.session, &self.presentes).await?;
        }
        let sorts = self
            .presentes
            .iter()
            .map(|uid| {
                let sort = if restees.contains(uid) {
                    Sort::Restee
                } else {
                    Sort::Partie(copie.nouveaux.get(uid).copied())
                };
                (*uid, sort)
            })
            .collect();
        Ok(Depot::Deplace {
            generation: copie.generation,
            sorts,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn relevee(reponse: &str) -> Copie {
        let (_, reponse) = Response::parse(reponse.as_bytes()).unwrap();
        let mut copie = Copie::default();
        copie.relever(&reponse);
        copie
    }

    #[test]
    fn releve_les_nouveaux_uid_dans_l_ordre_de_la_copie() {
        // GreenMail répète le début de chaque liste : les paires restent alignées.
        let greenmail = relevee("* OK [COPYUID 73 2,2:3 1,1:2] fait\r\n");
        assert_eq!(greenmail.generation, Some(73));
        assert_eq!(greenmail.nouveaux, BTreeMap::from([(2, 1), (3, 2)]));
        let etiquetee = relevee("a1 OK [COPYUID 9 304,319:320 3956:3958] fait\r\n");
        assert_eq!(etiquetee.generation, Some(9));
        assert_eq!(
            etiquetee.nouveaux,
            BTreeMap::from([(304, 3956), (319, 3957), (320, 3958)])
        );
    }

    #[test]
    fn ignore_une_reponse_sans_correspondance_sure() {
        for reponse in [
            "* OK [COPYUID 73 2:3 7] désalignée\r\n",
            "* OK [UIDNEXT 4] autre code\r\n",
            "* OK sans code\r\n",
            "* 3 EXPUNGE\r\n",
        ] {
            let copie = relevee(reponse);
            assert_eq!(copie.generation, None, "{reponse}");
            assert!(copie.nouveaux.is_empty(), "{reponse}");
        }
        // Une plage démesurée est coupée sans épuiser la mémoire.
        let longue = relevee("* OK [COPYUID 73 1:4000000000 5:4000000004] longue\r\n");
        assert_eq!(longue.nouveaux.len(), CORRESPONDANCES);
        assert_eq!(longue.nouveaux[&1], 5);
    }

    #[test]
    fn met_le_nom_du_dossier_entre_guillemets() {
        assert_eq!(boite("Archives").unwrap(), "\"Archives\"");
        assert_eq!(
            boite(r#"Mes "vieux" \ plis"#).unwrap(),
            r#""Mes \"vieux\" \\ plis""#
        );
        for chemin in ["Archives\r\nx LOGOUT", "Archives\nx", "Archives\rx"] {
            assert!(boite(chemin).is_err(), "{chemin:?}");
        }
    }
}
