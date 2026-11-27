use rusqlite::{Connection, OptionalExtension, Row, params};
use serde::Serialize;

use super::{Commande, Filtre};

const PAR_PAGE: usize = 20;
const LIMITE_SQL: i64 = PAR_PAGE as i64 + 1;

#[derive(Serialize)]
pub(super) struct Decision {
    pub adresse: String,
    pub etat: String,
    destination: String,
    pub revision: i64,
    date_decision: Option<i64>,
}

fn decision(ligne: &Row<'_>) -> rusqlite::Result<Decision> {
    Ok(Decision {
        adresse: ligne.get(0)?,
        etat: ligne.get(1)?,
        destination: ligne.get(2)?,
        revision: ligne.get(3)?,
        date_decision: ligne.get(4)?,
    })
}

fn chercher(db: &Connection, compte: i64, adresse: &str) -> rusqlite::Result<Option<Decision>> {
    db.query_row(
        "SELECT address,state,destination,revision,decided_at FROM sender_decisions
        WHERE account_id=?1 AND address=?2",
        params![compte, adresse],
        decision,
    )
    .optional()
}

#[derive(Serialize)]
pub(super) struct Lettre {
    uid: u32,
    generation: u32,
    fil: String,
    nom: Option<String>,
    objet: String,
    extrait: String,
    date: i64,
    non_lu: bool,
    etoile: bool,
    piece_jointe: bool,
    repondu: bool,
    transfere: bool,
}

#[derive(Serialize)]
pub(super) struct Suite {
    uid: u32,
    generation: u32,
}

#[derive(Serialize)]
pub(super) struct Lettres {
    messages: Vec<Lettre>,
    suivant: Option<Suite>,
}

pub(super) fn messages(
    db: &Connection,
    compte: i64,
    adresse: &str,
    avant: Option<(u32, u32)>,
) -> rusqlite::Result<Lettres> {
    let mut sql = String::from("SELECT uid,uidvalidity,thread_id,sender_name,subject,snippet,received_at,
        unread,starred,attached,answered,forwarded FROM screening_letters WHERE account_id=?1 AND sender_address=?2");
    if avant.is_some() {
        sql.push_str(" AND uid<?4 AND uidvalidity=?5");
    }
    sql.push_str(" ORDER BY uid DESC LIMIT ?3");
    let mut requete = db.prepare_cached(&sql)?;
    let lignes = match avant {
        Some((uid, generation)) => {
            requete.query(params![compte, adresse, LIMITE_SQL, uid, generation])?
        }
        None => requete.query(params![compte, adresse, LIMITE_SQL])?,
    };
    let mut messages = lignes
        .mapped(|r| {
            Ok(Lettre {
                uid: r.get(0)?,
                generation: r.get(1)?,
                fil: r.get(2)?,
                nom: r.get(3)?,
                objet: r.get(4)?,
                extrait: r.get(5)?,
                date: r.get(6)?,
                non_lu: r.get(7)?,
                etoile: r.get(8)?,
                piece_jointe: r.get(9)?,
                repondu: r.get(10)?,
                transfere: r.get(11)?,
            })
        })
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let suivant = if messages.len() > PAR_PAGE {
        messages.truncate(PAR_PAGE);
        messages.last().map(|m| Suite {
            uid: m.uid,
            generation: m.generation,
        })
    } else {
        None
    };
    Ok(Lettres { messages, suivant })
}

fn apercus(
    db: &Connection,
    compte: i64,
    adresse: &str,
) -> rusqlite::Result<Vec<crate::pages::Thread>> {
    db.prepare_cached("WITH classes AS (
       SELECT *, count(*) OVER fil AS total, max(unread) OVER fil AS non_lu, max(starred) OVER fil AS etoile,
           max(attached) OVER fil AS piece, max(answered) OVER fil AS repondu, max(forwarded) OVER fil AS transfere,
           row_number() OVER (PARTITION BY thread_id ORDER BY received_at DESC, uid DESC) AS rang
       FROM screening_letters WHERE account_id=?1 AND sender_address=?2 WINDOW fil AS (PARTITION BY thread_id))
       SELECT thread_id,subject,sender_name,sender_address,snippet,received_at,total,non_lu,etoile,piece,repondu,transfere
       FROM classes WHERE rang=1 ORDER BY received_at DESC,thread_id DESC LIMIT ?3")?
      .query_map(params![compte,adresse,PAR_PAGE as i64],crate::pages::lire_resume)?.collect()
}

#[derive(Serialize)]
pub(super) struct Expediteur {
    #[serde(flatten)]
    decision: Decision,
    apercus: Vec<crate::pages::Thread>,
    #[serde(flatten)]
    lettres: Lettres,
}

#[derive(Serialize)]
pub(super) struct Catalogue {
    compte: i64,
    revision: i64,
    expediteurs: Vec<Expediteur>,
    suite: Option<String>,
}

fn revision(db: &Connection, compte: i64) -> rusqlite::Result<i64> {
    db.query_row(
        "SELECT revision FROM account_revisions WHERE account_id=?1",
        [compte],
        |r| r.get(0),
    )
}

pub(super) fn catalogue(
    db: &Connection,
    compte: i64,
    filtre: Filtre,
    apres: &str,
) -> rusqlite::Result<Catalogue> {
    let condition = match filtre {
        Filtre::Attente => {
            "d.state='en_attente' AND EXISTS(SELECT 1 FROM list_threads p
            WHERE p.account_id=d.account_id AND p.scope='tri' AND p.item=d.address)"
        }
        Filtre::Historique => "d.state!='en_attente'",
    };
    let sql = format!(
        "SELECT d.address,d.state,d.destination,d.revision,d.decided_at FROM sender_decisions d
        WHERE d.account_id=?1 AND d.address>?2 AND {condition} ORDER BY d.address LIMIT ?3"
    );
    let mut choix = db
        .prepare_cached(&sql)?
        .query_map(params![compte, apres, LIMITE_SQL], decision)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let suite = if choix.len() > PAR_PAGE {
        choix.truncate(PAR_PAGE);
        choix.last().map(|d| d.adresse.clone())
    } else {
        None
    };
    let expediteurs = choix
        .into_iter()
        .map(|d| {
            let lettres = messages(db, compte, &d.adresse, None)?;
            let apercus = apercus(db, compte, &d.adresse)?;
            Ok(Expediteur {
                decision: d,
                apercus,
                lettres,
            })
        })
        .collect::<rusqlite::Result<_>>()?;
    Ok(Catalogue {
        compte,
        revision: revision(db, compte)?,
        expediteurs,
        suite,
    })
}

pub(super) fn correspondance(
    db: &Connection,
    compte: i64,
    adresse: &str,
    avant: Option<(u32, u32)>,
) -> rusqlite::Result<Option<Lettres>> {
    if chercher(db, compte, adresse)?.is_none() {
        return Ok(None);
    }
    messages(db, compte, adresse, avant).map(Some)
}

#[derive(Serialize)]
pub(super) struct Resultat {
    compte: i64,
    revision_compte: i64,
    decision: Decision,
}

pub(super) enum Application {
    Absente,
    Conflit(Resultat),
    Enregistree(Resultat),
}

pub(super) fn appliquer(
    db: &mut Connection,
    compte: i64,
    cmd: Commande,
) -> rusqlite::Result<Application> {
    let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let ecrit = tx.execute("UPDATE sender_decisions SET state=?3, destination=?4, revision=revision+1, decided_at=unixepoch()
        WHERE account_id=?1 AND address=?2 AND revision=?5",
        params![compte, cmd.adresse, cmd.decision.valeur(), cmd.destination.valeur(), cmd.revision_attendue])?;
    let Some(decision) = chercher(&tx, compte, &cmd.adresse)? else {
        return Ok(Application::Absente);
    };
    let resultat = Resultat {
        compte,
        revision_compte: revision(&tx, compte)?,
        decision,
    };
    tx.commit()?;
    Ok(if ecrit == 0 {
        Application::Conflit(resultat)
    } else {
        Application::Enregistree(resultat)
    })
}
