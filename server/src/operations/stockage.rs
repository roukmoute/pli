use std::collections::BTreeMap;

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

use super::deplacement::{Arrivee, archives, ranger};
use crate::{
    Error,
    db::Database,
    lecture::{Cle, enregistrer},
};

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Genre {
    Etoile,
    SansEtoile,
    Lu,
    NonLu,
    Archiver,
}

impl Genre {
    fn nom(self) -> &'static str {
        match self {
            Self::Etoile => "etoile",
            Self::SansEtoile => "sans_etoile",
            Self::Lu => "lu",
            Self::NonLu => "non_lu",
            Self::Archiver => "archiver",
        }
    }
}

#[derive(Deserialize)]
pub(super) struct Demande {
    pub id: String,
    #[serde(rename = "type")]
    pub genre: Genre,
    pub vue: Option<String>,
    pub cibles: Vec<String>,
}

#[derive(Serialize)]
struct Resultat {
    id: String,
    etat: String,
}

#[derive(Serialize)]
pub(super) struct Operation {
    id: String,
    #[serde(rename = "type")]
    genre: String,
    etat: String,
    revision: i64,
    resultats: Vec<Resultat>,
}

pub(super) fn lire(db: &Connection, compte: i64, id: &str) -> rusqlite::Result<Option<Operation>> {
    let connue: Option<(i64, String, String)> = db
        .query_row(
            "SELECT seq,kind,state FROM operations WHERE (account_id,id)=(?1,?2)",
            params![compte, id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?;
    let Some((seq, genre, etat)) = connue else {
        return Ok(None);
    };
    let revision = db.query_row(
        "SELECT revision FROM account_revisions WHERE account_id=?1",
        [compte],
        |r| r.get(0),
    )?;
    let resultats = db
        .prepare_cached(
            "SELECT target,result FROM operation_results WHERE operation=?1 ORDER BY position",
        )?
        .query_map([seq], |r| {
            Ok(Resultat {
                id: r.get(0)?,
                etat: r.get(1)?,
            })
        })?
        .collect::<rusqlite::Result<_>>()?;
    Ok(Some(Operation {
        id: id.to_owned(),
        genre,
        etat,
        revision,
        resultats,
    }))
}

// Une lettre déjà rangée dans le dossier d’arrivée n’a pas à être déplacée.
const LETTRES: &str =
    "INSERT INTO operation_messages(operation,position,folder_id,uidvalidity,uid,previous)
     SELECT o.seq,?2,m.folder_id,m.uidvalidity,m.uid,
            EXISTS(SELECT 1 FROM json_each(m.flags) WHERE lower(value)=lower(k.flag))
     FROM readable_messages m JOIN operations o ON o.seq=?1 AND o.account_id=m.account_id
     JOIN operation_kinds k ON k.kind=o.kind
     WHERE m.folder_id IS NOT o.destination AND ";

// Avec une vue, une cible désigne un fil et couvre ses lettres visibles dans cette vue.
// Sans vue, elle désigne une lettre. Le compte de l’opération borne les deux recherches.
fn viser(tx: &Connection, seq: i64, demande: &Demande) -> rusqlite::Result<()> {
    let mut cible = tx.prepare_cached("INSERT INTO operation_targets VALUES (?1,?2,?3)")?;
    let mut fil = tx.prepare_cached(&format!(
        "{LETTRES}m.thread_id=?3 AND ?4 IN (m.destination,'dossier/' || m.folder_id)"
    ))?;
    let mut lettre = tx.prepare_cached(&format!(
        "{LETTRES}(m.folder_id,m.uidvalidity,m.uid)=(?3,?4,?5)"
    ))?;
    for (position, id) in demande.cibles.iter().enumerate() {
        let position = position as i64;
        cible.execute(params![seq, position, id])?;
        if let Some(vue) = &demande.vue {
            fil.execute(params![seq, position, id, vue])?;
        } else if let Some(cle) = Cle::lire(id) {
            lettre.execute(params![seq, position, cle.dossier, cle.generation, cle.uid])?;
        }
    }
    Ok(())
}

/// Enregistre l’opération, sa projection et la révision du compte dans une seule transaction.
/// Sans dossier d’arrivée pour le déplacement demandé, rien n’est enregistré.
pub(super) fn creer(
    db: &Database,
    compte: i64,
    demande: &Demande,
) -> Result<Option<Operation>, Error> {
    let mut writer = db.writer()?;
    let tx = writer.transaction()?;
    // Un identifiant déjà connu rend son résultat sans rien réappliquer.
    if let Some(connue) = lire(&tx, compte, &demande.id)? {
        return Ok(Some(connue));
    }
    // Un déplacement fixe son dossier d’arrivée dès l’enregistrement.
    let destination = match demande.genre {
        Genre::Archiver => {
            let Some(dossier) = archives(&tx, compte)? else {
                return Ok(None);
            };
            Some(dossier)
        }
        _ => None,
    };
    tx.execute(
        "INSERT INTO operations(account_id,id,kind,destination) VALUES (?1,?2,?3,?4)",
        params![compte, demande.id, demande.genre.nom(), destination],
    )?;
    let seq = tx.last_insert_rowid();
    viser(&tx, seq, demande)?;
    tx.execute(
        "UPDATE messages SET flags=CASE WHEN k.expected THEN json_insert(messages.flags,'$[#]',k.flag)
           ELSE (SELECT json_group_array(value) FROM json_each(messages.flags) WHERE lower(value)!=lower(k.flag)) END
         FROM operations o JOIN operation_kinds k ON k.kind=o.kind
         WHERE o.seq=?1 AND messages.account_id=o.account_id
           AND (messages.folder_id,messages.uidvalidity,messages.uid) IN
               (SELECT folder_id,uidvalidity,uid FROM operation_messages WHERE operation=o.seq)
           AND EXISTS(SELECT 1 FROM json_each(messages.flags) WHERE lower(value)=lower(k.flag)) != k.expected",
        [seq],
    )?;
    // Une lettre à déplacer quitte ses listes dès l’enregistrement.
    tx.execute(
        "UPDATE messages SET moving=1 FROM operations o
         WHERE o.seq=?1 AND o.destination IS NOT NULL AND messages.account_id=o.account_id
           AND (messages.folder_id,messages.uidvalidity,messages.uid) IN
               (SELECT folder_id,uidvalidity,uid FROM operation_messages WHERE operation=o.seq)",
        [seq],
    )?;
    // Sans lettre atteinte, il n’y a rien à appliquer chez l’hébergeur.
    tx.execute(
        "UPDATE operations SET state='refusee' WHERE seq=?1
           AND NOT EXISTS (SELECT 1 FROM operation_messages WHERE operation=?1)",
        [seq],
    )?;
    let operation =
        lire(&tx, compte, &demande.id)?.ok_or("Opération illisible après son enregistrement.")?;
    tx.commit()?;
    Ok(Some(operation))
}

pub(crate) fn comptes_en_attente(db: &Database) -> Result<Vec<i64>, Error> {
    let reader = db.reader()?;
    let comptes = reader
        .prepare("SELECT DISTINCT account_id FROM operations WHERE state='en_attente'")?
        .query_map([], |r| r.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    Ok(comptes)
}

/// Ce que la messagerie doit faire des lettres d’un lot.
pub(crate) enum Action {
    Drapeau {
        drapeau: String,
        valeur: bool,
    },
    /// Sans arrivée quand le dossier prévu a disparu depuis l’enregistrement.
    Deplacement(Option<Arrivee>),
}

/// Lettres d’un même dossier à modifier par une seule série de commandes IMAP.
pub(crate) struct Lot {
    pub(super) seq: i64,
    pub(super) dossier: i64,
    pub generation: u32,
    /// Absent quand le dossier a disparu ou changé de génération.
    pub chemin: Option<String>,
    pub action: Action,
    pub uids: Vec<u32>,
}

/// Premier lot de la plus ancienne opération en attente du compte.
pub(crate) fn prochain(db: &Database, compte: i64) -> Result<Option<Lot>, Error> {
    let reader = db.reader()?;
    let premier = reader
        .query_row(
            "SELECT o.seq,m.folder_id,m.uidvalidity,f.remote_path,k.flag,k.expected,
                    a.id AS arrivee,a.uidvalidity AS generation_arrivee,
                    a.remote_path AS chemin_arrivee
             FROM operations o
             JOIN operation_kinds k ON k.kind=o.kind
             JOIN operation_messages m ON m.operation=o.seq AND m.result='en_attente'
             LEFT JOIN folders f ON f.id=m.folder_id AND f.uidvalidity=m.uidvalidity
             LEFT JOIN folders a ON a.id=o.destination AND a.uidvalidity IS NOT NULL
             WHERE o.account_id=?1 AND o.state='en_attente'
             ORDER BY o.seq,m.folder_id,m.uidvalidity LIMIT 1",
            [compte],
            |r| {
                let action = match r.get::<_, Option<String>>(4)? {
                    Some(drapeau) => Action::Drapeau {
                        drapeau,
                        valeur: r.get(5)?,
                    },
                    None => Action::Deplacement(Arrivee::lire(r)?),
                };
                Ok(Lot {
                    seq: r.get(0)?,
                    dossier: r.get(1)?,
                    generation: r.get(2)?,
                    chemin: r.get(3)?,
                    action,
                    uids: Vec::new(),
                })
            },
        )
        .optional()?;
    let Some(mut lot) = premier else {
        return Ok(None);
    };
    lot.uids = reader
        .prepare_cached(
            "SELECT DISTINCT uid FROM operation_messages
             WHERE operation=?1 AND (folder_id,uidvalidity)=(?2,?3) AND result='en_attente'
             ORDER BY uid LIMIT 100",
        )?
        .query_map(params![lot.seq, lot.dossier, lot.generation], |r| r.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    Ok(Some(lot))
}

/// Sort d’une lettre que la commande de déplacement a trouvée dans son dossier.
#[derive(Clone, Copy)]
pub(crate) enum Sort {
    /// Partie, avec son UID d’arrivée quand le serveur le communique.
    Partie(Option<u32>),
    Restee,
}

/// Réponse de l’hébergeur aux commandes d’un lot.
pub(crate) enum Depot {
    /// Pour chaque lettre rendue par le serveur, vrai si le drapeau a l’état attendu.
    Applique(BTreeMap<u32, bool>),
    /// Sort des lettres trouvées avant le déplacement. La génération est celle
    /// du dossier d’arrivée, annoncée avec les nouveaux UID.
    Deplace {
        generation: Option<u32>,
        sorts: BTreeMap<u32, Sort>,
    },
    Refuse,
}

#[derive(Clone, Copy)]
pub(super) enum Issue {
    Confirme,
    Refuse,
    Absent,
}

impl Issue {
    fn nom(self) -> &'static str {
        match self {
            Self::Confirme => "confirme",
            Self::Refuse => "refuse",
            Self::Absent => "absent",
        }
    }
}

impl Depot {
    fn issue(&self, uid: u32) -> Issue {
        match self {
            Self::Refuse => Issue::Refuse,
            Self::Applique(etats) => match etats.get(&uid) {
                Some(true) => Issue::Confirme,
                Some(false) => Issue::Refuse,
                None => Issue::Absent,
            },
            Self::Deplace { sorts, .. } => match sorts.get(&uid) {
                Some(Sort::Partie(_)) => Issue::Confirme,
                Some(Sort::Restee) => Issue::Refuse,
                None => Issue::Absent,
            },
        }
    }

    /// UID d’arrivée de la lettre, s’il appartient à la génération connue du dossier.
    pub(super) fn rattachement(&self, uid: u32, connue: u32) -> Option<u32> {
        match self {
            Self::Deplace { generation, sorts } if *generation == Some(connue) => {
                match sorts.get(&uid) {
                    Some(Sort::Partie(nouveau)) => *nouveau,
                    _ => None,
                }
            }
            _ => None,
        }
    }
}

// Une intention plus récente sur la même lettre et le même drapeau garde la main :
// elle hérite de l’état d’avant. Sinon la projection retrouve cet état.
fn retablir(
    tx: &Connection,
    compte: i64,
    lot: &Lot,
    drapeau: &str,
    uid: u32,
) -> rusqlite::Result<()> {
    let lettre = params![lot.seq, lot.dossier, lot.generation, uid];
    let precedent: bool = tx.query_row(
        "SELECT previous FROM operation_messages
         WHERE operation=?1 AND (folder_id,uidvalidity,uid)=(?2,?3,?4) LIMIT 1",
        lettre,
        |r| r.get(0),
    )?;
    let suivante: Option<i64> = tx.query_row(
        "SELECT min(o.seq) FROM operations o
         JOIN operation_kinds k ON k.kind=o.kind
         JOIN operation_messages m ON m.operation=o.seq AND m.result='en_attente'
         JOIN operations refusee ON refusee.seq=?1
         JOIN operation_kinds kr ON kr.kind=refusee.kind
         WHERE o.seq>refusee.seq AND o.account_id=refusee.account_id AND k.flag=kr.flag
           AND o.state='en_attente' AND (m.folder_id,m.uidvalidity,m.uid)=(?2,?3,?4)",
        lettre,
        |r| r.get(0),
    )?;
    let Some(suivante) = suivante else {
        let cle = Cle {
            dossier: lot.dossier,
            generation: lot.generation,
            uid,
        };
        return enregistrer(tx, compte, cle, drapeau, precedent);
    };
    tx.execute(
        "UPDATE operation_messages SET previous=?5
         WHERE operation=?1 AND (folder_id,uidvalidity,uid)=(?2,?3,?4)",
        params![suivante, lot.dossier, lot.generation, uid, precedent],
    )?;
    Ok(())
}

/// Inscrit le résultat de chaque lettre du lot, rétablit les refus puis clôt l’opération
/// quand plus aucune de ses lettres n’attend.
pub(crate) fn conclure(db: &Database, compte: i64, lot: &Lot, depot: &Depot) -> Result<(), Error> {
    let mut writer = db.writer()?;
    let tx = writer.transaction()?;
    for &uid in &lot.uids {
        let issue = depot.issue(uid);
        // Une lettre disparue après une commande déjà envoyée est partie avec elle.
        tx.execute(
            "UPDATE operation_messages
             SET result=CASE WHEN attempted AND ?5='absent' THEN 'confirme' ELSE ?5 END
             WHERE operation=?1 AND (folder_id,uidvalidity,uid)=(?2,?3,?4)",
            params![lot.seq, lot.dossier, lot.generation, uid, issue.nom()],
        )?;
        match (&lot.action, issue) {
            (Action::Drapeau { .. }, Issue::Confirme) => {}
            (Action::Drapeau { drapeau, .. }, _) => retablir(&tx, compte, lot, drapeau, uid)?,
            (Action::Deplacement(arrivee), issue) => {
                ranger(&tx, compte, lot, uid, issue, arrivee.as_ref(), depot)?
            }
        }
    }
    tx.execute(
        "UPDATE operations SET state=CASE WHEN EXISTS
             (SELECT 1 FROM operation_messages WHERE operation=?1 AND result='confirme')
             THEN 'confirmee' ELSE 'refusee' END
         WHERE seq=?1 AND NOT EXISTS
             (SELECT 1 FROM operation_messages WHERE operation=?1 AND result='en_attente')",
        [lot.seq],
    )?;
    tx.commit()?;
    Ok(())
}
