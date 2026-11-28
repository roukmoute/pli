use rusqlite::{Connection, params};

use super::Cle;

/// Drapeau IMAP d’une lettre lue.
pub(crate) const LU: &str = "\\Seen";

/// Pose ou retire un drapeau dans la projection d’une lettre.
pub(crate) fn enregistrer(
    db: &Connection,
    compte: i64,
    cle: Cle,
    drapeau: &str,
    valeur: bool,
) -> rusqlite::Result<()> {
    db.prepare_cached(
        "UPDATE messages SET flags=CASE WHEN ?6 THEN json_insert(flags,'$[#]',?5)
           ELSE (SELECT json_group_array(value) FROM json_each(messages.flags) WHERE lower(value)!=lower(?5)) END
         WHERE (account_id,folder_id,uidvalidity,uid)=(?1,?2,?3,?4)
           AND EXISTS(SELECT 1 FROM json_each(flags) WHERE lower(value)=lower(?5)) != ?6"
    )?.execute(params![compte, cle.dossier, cle.generation, cle.uid, drapeau, valeur])?;
    Ok(())
}
