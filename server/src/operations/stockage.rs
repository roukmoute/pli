use std::collections::BTreeMap;

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

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
}

impl Genre {
    fn nom(self) -> &'static str {
        match self {
            Self::Etoile => "etoile",
            Self::SansEtoile => "sans_etoile",
            Self::Lu => "lu",
            Self::NonLu => "non_lu",
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

// Avec une vue, une cible désigne un fil et couvre ses lettres visibles dans cette vue.
// Sans vue, elle désigne une lettre. Le compte de l’opération borne les deux recherches.
fn viser(tx: &Connection, seq: i64, demande: &Demande) -> rusqlite::Result<()> {
    let mut cible = tx.prepare_cached("INSERT INTO operation_targets VALUES (?1,?2,?3)")?;
    let mut fil = tx.prepare_cached(
        "INSERT INTO operation_messages(operation,position,folder_id,uidvalidity,uid,previous)
         SELECT o.seq,?2,m.folder_id,m.uidvalidity,m.uid,
                EXISTS(SELECT 1 FROM json_each(m.flags) WHERE lower(value)=lower(o.flag))
         FROM readable_messages m JOIN operations o ON o.seq=?1 AND o.account_id=m.account_id
         WHERE m.thread_id=?3 AND ?4 IN (m.destination,'dossier/' || m.folder_id)",
    )?;
    let mut lettre = tx.prepare_cached(
        "INSERT INTO operation_messages(operation,position,folder_id,uidvalidity,uid,previous)
         SELECT o.seq,?2,m.folder_id,m.uidvalidity,m.uid,
                EXISTS(SELECT 1 FROM json_each(m.flags) WHERE lower(value)=lower(o.flag))
         FROM readable_messages m JOIN operations o ON o.seq=?1 AND o.account_id=m.account_id
         WHERE (m.folder_id,m.uidvalidity,m.uid)=(?3,?4,?5)",
    )?;
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
pub(super) fn creer(db: &Database, compte: i64, demande: &Demande) -> Result<Operation, Error> {
    let mut writer = db.writer()?;
    let tx = writer.transaction()?;
    // Un identifiant déjà connu rend son résultat sans rien réappliquer.
    if let Some(connue) = lire(&tx, compte, &demande.id)? {
        return Ok(connue);
    }
    tx.execute(
        "INSERT INTO operations(account_id,id,kind) VALUES (?1,?2,?3)",
        params![compte, demande.id, demande.genre.nom()],
    )?;
    let seq = tx.last_insert_rowid();
    viser(&tx, seq, demande)?;
    tx.execute(
        "UPDATE messages SET flags=CASE WHEN o.expected THEN json_insert(messages.flags,'$[#]',o.flag)
           ELSE (SELECT json_group_array(value) FROM json_each(messages.flags) WHERE lower(value)!=lower(o.flag)) END
         FROM operations o
         WHERE o.seq=?1 AND messages.account_id=o.account_id
           AND (messages.folder_id,messages.uidvalidity,messages.uid) IN
               (SELECT folder_id,uidvalidity,uid FROM operation_messages WHERE operation=o.seq)
           AND EXISTS(SELECT 1 FROM json_each(messages.flags) WHERE lower(value)=lower(o.flag)) != o.expected",
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
    Ok(operation)
}

pub(crate) fn comptes_en_attente(db: &Database) -> Result<Vec<i64>, Error> {
    let reader = db.reader()?;
    let comptes = reader
        .prepare("SELECT DISTINCT account_id FROM operations WHERE state='en_attente'")?
        .query_map([], |r| r.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    Ok(comptes)
}

/// Lettres d’un même dossier à modifier par une seule commande IMAP.
pub(crate) struct Lot {
    seq: i64,
    dossier: i64,
    pub generation: u32,
    /// Absent quand le dossier a disparu ou changé de génération.
    pub chemin: Option<String>,
    pub drapeau: String,
    pub valeur: bool,
    pub uids: Vec<u32>,
}

/// Premier lot de la plus ancienne opération en attente du compte.
pub(crate) fn prochain(db: &Database, compte: i64) -> Result<Option<Lot>, Error> {
    let reader = db.reader()?;
    let premier = reader
        .query_row(
            "SELECT o.seq,m.folder_id,m.uidvalidity,f.remote_path,o.flag,o.expected
             FROM operations o
             JOIN operation_messages m ON m.operation=o.seq AND m.result='en_attente'
             LEFT JOIN folders f ON f.id=m.folder_id AND f.uidvalidity=m.uidvalidity
             WHERE o.account_id=?1 AND o.state='en_attente'
             ORDER BY o.seq,m.folder_id,m.uidvalidity LIMIT 1",
            [compte],
            |r| {
                Ok(Lot {
                    seq: r.get(0)?,
                    dossier: r.get(1)?,
                    generation: r.get(2)?,
                    chemin: r.get(3)?,
                    drapeau: r.get(4)?,
                    valeur: r.get(5)?,
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

/// Réponse de l’hébergeur à la commande d’un lot.
pub(crate) enum Depot {
    /// Pour chaque lettre rendue par le serveur, vrai si le drapeau a l’état attendu.
    Applique(BTreeMap<u32, bool>),
    Refuse,
}

#[derive(Clone, Copy)]
enum Issue {
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
        }
    }
}

// Une intention plus récente sur la même lettre et le même drapeau garde la main :
// elle hérite de l’état d’avant. Sinon la projection retrouve cet état.
fn retablir(tx: &Connection, compte: i64, lot: &Lot, uid: u32) -> rusqlite::Result<()> {
    let lettre = params![lot.seq, lot.dossier, lot.generation, uid];
    let precedent: bool = tx.query_row(
        "SELECT previous FROM operation_messages
         WHERE operation=?1 AND (folder_id,uidvalidity,uid)=(?2,?3,?4) LIMIT 1",
        lettre,
        |r| r.get(0),
    )?;
    let suivante: Option<i64> = tx.query_row(
        "SELECT min(o.seq) FROM operations o
         JOIN operation_messages m ON m.operation=o.seq AND m.result='en_attente'
         JOIN operations refusee ON refusee.seq=?1
         WHERE o.seq>refusee.seq AND o.account_id=refusee.account_id AND o.flag=refusee.flag
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
        return enregistrer(tx, compte, cle, &lot.drapeau, precedent);
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
        tx.execute(
            "UPDATE operation_messages SET result=?5
             WHERE operation=?1 AND (folder_id,uidvalidity,uid)=(?2,?3,?4)",
            params![lot.seq, lot.dossier, lot.generation, uid, issue.nom()],
        )?;
        match issue {
            Issue::Confirme => {}
            Issue::Refuse | Issue::Absent => retablir(&tx, compte, lot, uid)?,
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
