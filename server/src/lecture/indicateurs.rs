use std::{collections::BTreeMap, sync::Arc};

use axum::{
    Extension, Json,
    extract::{State, rejection::JsonRejection},
    http::StatusCode,
};
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};

use super::{Cle, sqlite, stockage};
use crate::{
    Error,
    db::Database,
    http::{ApiError, api_error},
    session::CompteAuthentifie,
    sync::Lecteur,
};

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Indicateur {
    Lu,
    Etoile,
}

impl Indicateur {
    pub(crate) fn imap(self) -> &'static str {
        match self {
            Self::Lu => "\\Seen",
            Self::Etoile => "\\Flagged",
        }
    }
}

#[derive(Deserialize)]
pub(super) struct Action {
    ids: Vec<String>,
    vue: Option<String>,
    indicateur: Indicateur,
    valeur: bool,
}

type Dossier = (i64, u32, String);
type Destinataires = BTreeMap<u32, Vec<usize>>;
type Lots = BTreeMap<Dossier, Destinataires>;

fn cibles(db: &Connection, compte: i64, action: &Action) -> rusqlite::Result<Lots> {
    let mut lots = Lots::new();
    let mut fils = db.prepare_cached(
        "SELECT folder_id,uidvalidity,remote_path,uid FROM readable_messages
         WHERE account_id=?1 AND thread_id=?2 AND ?3 IN (destination,'dossier/' || folder_id)",
    )?;
    for (rang, id) in action.ids.iter().enumerate() {
        if let Some(vue) = &action.vue {
            let lignes = fils.query_map(params![compte, id, vue], |r| {
                Ok(((r.get(0)?, r.get(1)?, r.get(2)?), r.get(3)?))
            })?;
            for ligne in lignes {
                let (dossier, uid) = ligne?;
                lots.entry(dossier)
                    .or_default()
                    .entry(uid)
                    .or_default()
                    .push(rang);
            }
        } else if let Some(cle) = Cle::lire(id)
            && let Some(chemin) = stockage::chemin(db, compte, cle)?
        {
            lots.entry((cle.dossier, cle.generation, chemin))
                .or_default()
                .entry(cle.uid)
                .or_default()
                .push(rang);
        }
    }
    Ok(lots)
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Debug)]
#[serde(rename_all = "snake_case")]
enum Etat {
    Confirme,
    Absent,
    NonConfirme,
    Partiel,
}

fn bilan(etats: &[Etat]) -> Etat {
    if !etats.contains(&Etat::Confirme) {
        return if etats.contains(&Etat::NonConfirme) {
            Etat::NonConfirme
        } else {
            Etat::Absent
        };
    }
    if etats.iter().all(|&e| e == Etat::Confirme) {
        Etat::Confirme
    } else {
        Etat::Partiel
    }
}

#[derive(Serialize)]
pub(super) struct Resultat {
    id: String,
    etat: Etat,
}

fn confirmer(
    db: &Database,
    compte: i64,
    dossier: i64,
    generation: u32,
    action: &Action,
    mut etats: BTreeMap<u32, Etat>,
) -> Result<BTreeMap<u32, Etat>, Error> {
    let mut writer = db.writer()?;
    let tx = writer.transaction()?;
    for (&uid, etat) in &mut etats {
        if *etat != Etat::Confirme {
            continue;
        }
        let cle = Cle {
            dossier,
            generation,
            uid,
        };
        if stockage::chemin(&tx, compte, cle)?.is_some() {
            enregistrer(&tx, compte, cle, action.indicateur, action.valeur)?;
        } else {
            *etat = Etat::Absent;
        }
    }
    tx.commit()?;
    Ok(etats)
}

pub(super) fn enregistrer(
    db: &Connection,
    compte: i64,
    cle: Cle,
    indicateur: Indicateur,
    valeur: bool,
) -> rusqlite::Result<()> {
    db.prepare_cached(
        "UPDATE messages SET flags=CASE WHEN ?6 THEN json_insert(flags,'$[#]',?5)
           ELSE (SELECT json_group_array(value) FROM json_each(messages.flags) WHERE lower(value)!=lower(?5)) END
         WHERE (account_id,folder_id,uidvalidity,uid)=(?1,?2,?3,?4)
           AND EXISTS(SELECT 1 FROM json_each(flags) WHERE lower(value)=lower(?5)) != ?6"
    )?.execute(params![compte, cle.dossier, cle.generation, cle.uid, indicateur.imap(), valeur])?;
    Ok(())
}

async fn traiter(
    db: &Arc<Database>,
    lecteur: &Lecteur,
    compte: i64,
    dossier: &Dossier,
    uids: &[u32],
    action: Arc<Action>,
) -> Result<BTreeMap<u32, Etat>, ApiError> {
    let (id, generation, chemin) = dossier;
    let reponse = lecteur
        .marquer(
            compte,
            *generation,
            chemin,
            uids,
            action.indicateur,
            action.valeur,
        )
        .await;
    let etats = uids
        .iter()
        .map(|&uid| {
            let etat = match &reponse {
                Err(_) => Etat::NonConfirme,
                Ok(flags) => match flags.get(&uid) {
                    Some(true) => Etat::Confirme,
                    Some(false) => Etat::NonConfirme,
                    None => Etat::Absent,
                },
            };
            (uid, etat)
        })
        .collect();
    let (id, generation) = (*id, *generation);
    sqlite(db.clone(), move |db| {
        confirmer(db, compte, id, generation, &action, etats)
    })
    .await
}

pub(super) async fn modifier(
    compte: CompteAuthentifie,
    State(db): State<Arc<Database>>,
    Extension(lecteur): Extension<Lecteur>,
    entree: Result<Json<Action>, JsonRejection>,
) -> Result<Json<Vec<Resultat>>, ApiError> {
    let Json(action) = entree.map_err(|e| {
        api_error(
            e.status(),
            "indicateur_invalide",
            "Indicateur ou sélection invalide.",
        )
    })?;
    if !(1..=100).contains(&action.ids.len()) {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "lot_invalide",
            "Sélectionnez de 1 à 100 éléments.",
        ));
    }
    let compte = compte.account_id;
    let _ecriture = lecteur.ecriture(compte).await;
    let action = Arc::new(action);
    let cible = action.clone();
    let lots = sqlite(db.clone(), move |db| {
        Ok(cibles(&*db.reader()?, compte, &cible)?)
    })
    .await?;
    let mut etats = vec![Vec::new(); action.ids.len()];
    for (dossier, membres) in lots {
        let uids: Vec<_> = membres.keys().copied().collect();
        for groupe in uids.chunks(100) {
            for (uid, etat) in
                traiter(&db, &lecteur, compte, &dossier, groupe, action.clone()).await?
            {
                for &rang in &membres[&uid] {
                    etats[rang].push(etat);
                }
            }
        }
    }
    Ok(Json(
        action
            .ids
            .iter()
            .zip(etats)
            .map(|(id, etats)| Resultat {
                id: id.clone(),
                etat: bilan(&etats),
            })
            .collect(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TestApp;

    #[test]
    fn distingue_les_refus_et_le_succes_partiel() {
        use Etat::{Absent, Confirme, NonConfirme, Partiel};
        for (lettres, attendu) in [
            (vec![], Absent),
            (vec![Absent], Absent),
            (vec![Absent, NonConfirme], NonConfirme),
            (vec![Confirme, Confirme], Confirme),
            (vec![Confirme, Absent], Partiel),
            (vec![NonConfirme, Confirme], Partiel),
        ] {
            assert_eq!(bilan(&lettres), attendu);
        }
    }

    #[test]
    fn une_lettre_disparue_avant_la_confirmation_reste_absente() -> Result<(), Error> {
        let app = TestApp::new()?;
        let action = Action {
            ids: vec!["1-1-3".into()],
            vue: None,
            indicateur: Indicateur::Lu,
            valeur: true,
        };
        let etats = BTreeMap::from([(3, Etat::Confirme), (8, Etat::NonConfirme)]);
        assert_eq!(
            confirmer(&app.db, 1, 1, 1, &action, etats)?,
            BTreeMap::from([(3, Etat::Absent), (8, Etat::NonConfirme)])
        );
        Ok(())
    }
}
