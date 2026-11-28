//! Projection d’un déplacement : dossier d’arrivée, trace de l’envoi et sort de chaque lettre.

use rusqlite::{Connection, OptionalExtension, Row, params};

use super::stockage::{Depot, Issue, Lot};
use crate::{Error, db::Database};

/// Dossier qui reçoit les lettres d’un déplacement.
pub(crate) struct Arrivee {
    dossier: i64,
    generation: u32,
    pub chemin: String,
}

impl Arrivee {
    pub(super) fn lire(ligne: &Row<'_>) -> rusqlite::Result<Option<Self>> {
        let Some(dossier) = ligne.get("arrivee")? else {
            return Ok(None);
        };
        Ok(Some(Self {
            dossier,
            generation: ligne.get("generation_arrivee")?,
            chemin: ligne.get("chemin_arrivee")?,
        }))
    }
}

/// Dossier d’archives du compte : celui que l’utilisateur a choisi, sinon celui du rôle découvert.
pub(super) fn archives(db: &Connection, compte: i64) -> rusqlite::Result<Option<i64>> {
    let dossier = db
        .query_row(
            "SELECT folder_id FROM archive_folders WHERE account_id=?1",
            [compte],
            |r| r.get(0),
        )
        .optional()?;
    Ok(dossier.flatten())
}

/// Note que la commande de déplacement part pour ces lettres. Après une coupure,
/// celles qui ont quitté leur dossier auront été déplacées par cette commande.
pub(crate) fn tenter(db: &Database, lot: &Lot, uids: &[u32]) -> Result<(), Error> {
    db.writer()?.execute(
        "UPDATE operation_messages SET attempted=1
         WHERE operation=?1 AND (folder_id,uidvalidity)=(?2,?3)
           AND uid IN (SELECT value FROM json_each(?4))",
        params![
            lot.seq,
            lot.dossier,
            lot.generation,
            serde_json::to_string(uids)?
        ],
    )?;
    Ok(())
}

/// Range la lettre selon son sort : rendue à ses listes après un refus, rattachée
/// à son dossier d’arrivée quand son nouvel UID est connu, retirée sinon.
pub(super) fn ranger(
    tx: &Connection,
    compte: i64,
    lot: &Lot,
    uid: u32,
    issue: Issue,
    arrivee: Option<&Arrivee>,
    depot: &Depot,
) -> rusqlite::Result<()> {
    let lettre = params![compte, lot.dossier, lot.generation, uid];
    if let Issue::Refuse = issue {
        tx.execute(
            "UPDATE messages SET moving=0
             WHERE (account_id,folder_id,uidvalidity,uid)=(?1,?2,?3,?4)",
            lettre,
        )?;
        return Ok(());
    }
    if let Some(arrivee) = arrivee
        && let Some(nouveau) = depot.rattachement(uid, arrivee.generation)
    {
        // Le rendu gardé en cache porte l’ancienne clé de la lettre.
        tx.execute(
            "DELETE FROM message_bodies
             WHERE (account_id,folder_id,uidvalidity,uid)=(?1,?2,?3,?4)",
            lettre,
        )?;
        tx.execute(
            "UPDATE OR IGNORE messages SET folder_id=?5,uidvalidity=?6,uid=?7,moving=0
             WHERE (account_id,folder_id,uidvalidity,uid)=(?1,?2,?3,?4)",
            params![
                compte,
                lot.dossier,
                lot.generation,
                uid,
                arrivee.dossier,
                arrivee.generation,
                nouveau
            ],
        )?;
    }
    // Sans nouvel UID, la lettre reviendra avec l’import de son dossier d’arrivée.
    // Déjà connue à l’arrivée, elle n’a plus besoin de sa ligne de départ.
    tx.execute(
        "DELETE FROM messages WHERE (account_id,folder_id,uidvalidity,uid)=(?1,?2,?3,?4)",
        lettre,
    )?;
    Ok(())
}
