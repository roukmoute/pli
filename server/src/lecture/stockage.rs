use std::collections::BTreeMap;

use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;

use super::rendu::{Contenu, ImageMime, VERSION};
use crate::{Error, db::Database};

#[derive(Clone, Copy)]
pub(crate) struct Cle {
    pub dossier: i64,
    pub generation: u32,
    pub uid: u32,
}

impl Cle {
    pub fn lire(id: &str) -> Option<Self> {
        let (dossier, fin) = id.split_once('-')?;
        let (generation, uid) = fin.split_once('-')?;
        Some(Self {
            dossier: dossier.parse().ok()?,
            generation: generation.parse().ok()?,
            uid: uid.parse().ok()?,
        })
    }
}

#[derive(Serialize)]
pub(super) struct Entete {
    id: String,
    objet: String,
    nom: Option<String>,
    adresse: Option<String>,
    destinataires: serde_json::Value,
    date: i64,
    non_lu: bool,
}

pub(super) fn fil(db: &Database, compte: i64, id: String) -> Result<Vec<Entete>, Error> {
    let reader = db.reader()?;
    let mut query = reader.prepare_cached(
        "SELECT printf('%d-%d-%d',folder_id,uidvalidity,uid),subject,sender_name,sender_address,
         recipients,received_at,NOT EXISTS(SELECT 1 FROM json_each(flags) WHERE lower(value)='\\seen')
         FROM readable_messages WHERE account_id=?1 AND thread_id=?2
         ORDER BY received_at,folder_id,uidvalidity,uid"
    )?;
    let entetes = query
        .query_map(params![compte, id], |r| {
            let recipients: String = r.get(4)?;
            let destinataires = serde_json::from_str(&recipients).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    4,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })?;
            Ok(Entete {
                id: r.get(0)?,
                objet: r.get(1)?,
                nom: r.get(2)?,
                adresse: r.get(3)?,
                destinataires,
                date: r.get(5)?,
                non_lu: r.get(6)?,
            })
        })?
        .collect::<rusqlite::Result<_>>()?;
    Ok(entetes)
}

pub(super) struct Cible {
    pub chemin: String,
    pub contenu: Option<Contenu>,
}

pub(super) fn cible(db: &Database, compte: i64, cle: Cle) -> Result<Option<Cible>, Error> {
    let reader = db.reader()?;
    let row: Option<(String, Option<String>)> = reader.query_row(
        "SELECT m.remote_path,b.content FROM readable_messages m LEFT JOIN message_bodies b
         ON (b.account_id,b.folder_id,b.uidvalidity,b.uid)=(m.account_id,m.folder_id,m.uidvalidity,m.uid) AND b.render_version=?5
         WHERE (m.account_id,m.folder_id,m.uidvalidity,m.uid)=(?1,?2,?3,?4)",
        params![compte, cle.dossier, cle.generation, cle.uid, VERSION],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )
    .optional()?;
    row.map(|(chemin, content)| {
        Ok(Cible {
            chemin,
            contenu: content.map(|s| serde_json::from_str(&s)).transpose()?,
        })
    })
    .transpose()
}

pub(super) fn chemin(conn: &Connection, compte: i64, cle: Cle) -> rusqlite::Result<Option<String>> {
    conn.query_row(
        "SELECT remote_path FROM readable_messages WHERE (account_id,folder_id,uidvalidity,uid)=(?1,?2,?3,?4)",
        params![compte, cle.dossier, cle.generation, cle.uid],
        |r| r.get(0),
    ).optional()
}

pub(super) fn conserver(
    db: &Database,
    compte: i64,
    cle: Cle,
    contenu: &Contenu,
    images: &[ImageMime],
) -> Result<bool, Error> {
    let mut writer = db.writer()?;
    let tx = writer.transaction()?;
    if chemin(&tx, compte, cle)?.is_none() {
        return Ok(false);
    }
    tx.execute(
        "INSERT OR REPLACE INTO message_bodies VALUES (?1,?2,?3,?4,?5,?6)",
        params![
            compte,
            cle.dossier,
            cle.generation,
            cle.uid,
            VERSION,
            serde_json::to_string(contenu)?
        ],
    )?;
    {
        let mut insert =
            tx.prepare_cached("INSERT INTO message_images VALUES (?1,?2,?3,?4,?5,?6,?7)")?;
        for image in images {
            insert.execute(params![
                compte,
                cle.dossier,
                cle.generation,
                cle.uid,
                image.partie,
                image.mime,
                image.octets
            ])?;
        }
    }
    tx.commit()?;
    Ok(true)
}

pub(super) fn image(
    db: &Database,
    compte: i64,
    cle: Cle,
    partie: u32,
) -> Result<Option<(String, Vec<u8>)>, Error> {
    Ok(db.reader()?.query_row(
        "SELECT mime,bytes FROM readable_messages JOIN message_bodies USING(account_id,folder_id,uidvalidity,uid)
         JOIN message_images USING(account_id,folder_id,uidvalidity,uid)
         WHERE (account_id,folder_id,uidvalidity,uid,part,render_version)=(?1,?2,?3,?4,?5,?6)",
        params![compte, cle.dossier, cle.generation, cle.uid, partie, VERSION],
        |r| Ok((r.get(0)?, r.get(1)?)),
    ).optional()?)
}

#[derive(Serialize)]
pub(super) struct Lecture {
    fil: String,
    revision: i64,
    non_lus: BTreeMap<String, bool>,
}

pub(super) fn confirmer(db: &Database, compte: i64, cle: Cle) -> Result<Option<Lecture>, Error> {
    let mut writer = db.writer()?;
    let tx = writer.transaction()?;
    let key = params![compte, cle.dossier, cle.generation, cle.uid];
    let fil: Option<String> = tx.query_row(
        "SELECT thread_id FROM readable_messages WHERE (account_id,folder_id,uidvalidity,uid)=(?1,?2,?3,?4)",
        key,
        |r| r.get(0),
    ).optional()?;
    let Some(fil) = fil else { return Ok(None) };
    super::indicateurs::enregistrer(&tx, compte, cle, super::Indicateur::Lu, true)?;
    let revision = tx.query_row(
        "SELECT revision FROM account_revisions WHERE account_id=?1",
        [compte],
        |r| r.get(0),
    )?;
    let non_lus = tx
        .prepare_cached("SELECT scope,unread FROM list_threads WHERE account_id=?1 AND item=?2")?
        .query_map(params![compte, fil], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<rusqlite::Result<_>>()?;
    tx.commit()?;
    Ok(Some(Lecture {
        fil,
        revision,
        non_lus,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TestApp;

    #[test]
    fn une_lettre_disparue_pendant_le_telechargement_ne_remplit_pas_le_cache() -> Result<(), Error>
    {
        let app = TestApp::new()?;
        let cle = Cle::lire("1-2-3").unwrap();
        let (contenu, images) = super::super::rendu::analyser(b"Subject: Exemple\r\n\r\nTexte")?;
        assert!(!conserver(&app.db, 1, cle, &contenu, &images)?);
        assert!(confirmer(&app.db, 1, cle)?.is_none());
        assert!(Cle::lire("inconnu").is_none());
        Ok(())
    }
}
