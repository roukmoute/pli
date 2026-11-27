mod stockage;

use std::sync::Arc;

use crate::{
    Error,
    db::Database,
    http::{ApiError, api_error, unavailable},
    session::CompteAuthentifie,
    sync,
};
use axum::{
    Json, Router,
    extract::{Query, State, rejection::JsonRejection},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use rusqlite::Connection;
use serde::Deserialize;

pub(crate) fn router() -> Router<Arc<Database>> {
    Router::new()
        .route("/v1/screener", get(attente))
        .route("/v1/screener/historique", get(historique))
        .route("/v1/screener/messages", get(messages))
        .route("/v1/screener/decisions", post(decider))
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum Choix {
    Accepte,
    Refuse,
}
impl Choix {
    fn valeur(&self) -> &'static str {
        match self {
            Self::Accepte => "accepte",
            Self::Refuse => "refuse",
        }
    }
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Destination {
    #[default]
    Reception,
    Flux,
    Justificatifs,
}
impl Destination {
    fn valeur(&self) -> &'static str {
        match self {
            Self::Reception => "reception",
            Self::Flux => "flux",
            Self::Justificatifs => "justificatifs",
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Commande {
    adresse: String,
    decision: Choix,
    #[serde(default)]
    destination: Destination,
    revision_attendue: i64,
}

#[derive(Deserialize)]
struct Navigation {
    #[serde(default)]
    details: bool,
    #[serde(default)]
    apres: String,
}

#[derive(Deserialize)]
struct Pagination {
    adresse: String,
    avant: Option<u32>,
    generation: Option<u32>,
}

#[derive(Clone, Copy)]
enum Filtre {
    Attente,
    Historique,
}

fn absente() -> ApiError {
    api_error(
        StatusCode::NOT_FOUND,
        "expediteur_introuvable",
        "Aucune décision pour cet expéditeur.",
    )
}

fn invalide() -> ApiError {
    api_error(
        StatusCode::BAD_REQUEST,
        "tri_invalide",
        "Adresse, décision, révision ou page de tri invalide.",
    )
}

fn adresse(s: &str) -> Result<String, ApiError> {
    let (local, domaine) = s.trim().rsplit_once('@').ok_or_else(invalide)?;
    if local.is_empty() || domaine.is_empty() || s.chars().any(char::is_control) {
        return Err(invalide());
    }
    Ok(format!("{local}@{}", domaine.to_ascii_lowercase()))
}

async fn lire<T: Send + 'static>(
    db: Arc<Database>,
    f: impl FnOnce(&Connection) -> rusqlite::Result<T> + Send + 'static,
) -> Result<Json<T>, ApiError> {
    tokio::task::spawn_blocking(move || -> Result<T, Error> {
        let mut lecteur = db.reader()?;
        let tx = lecteur.transaction()?;
        let valeur = f(&tx)?;
        tx.commit()?;
        Ok(valeur)
    })
    .await
    .map_err(unavailable)?
    .map(Json)
    .map_err(unavailable)
}

async fn attente(
    compte: CompteAuthentifie,
    State(db): State<Arc<Database>>,
    Query(nav): Query<Navigation>,
) -> Result<Response, ApiError> {
    if !nav.details {
        return Ok(sync::screener(compte, State(db)).await?.into_response());
    }
    Ok(lire(db, move |tx| {
        stockage::catalogue(tx, compte.account_id, Filtre::Attente, &nav.apres)
    })
    .await?
    .into_response())
}

async fn historique(
    compte: CompteAuthentifie,
    State(db): State<Arc<Database>>,
    Query(nav): Query<Navigation>,
) -> Result<Json<stockage::Catalogue>, ApiError> {
    lire(db, move |tx| {
        stockage::catalogue(tx, compte.account_id, Filtre::Historique, &nav.apres)
    })
    .await
}

async fn messages(
    compte: CompteAuthentifie,
    State(db): State<Arc<Database>>,
    Query(page): Query<Pagination>,
) -> Result<Json<stockage::Lettres>, ApiError> {
    let adresse = adresse(&page.adresse)?;
    let avant = match (page.avant, page.generation) {
        (Some(uid), Some(generation)) => Some((uid, generation)),
        (None, None) => None,
        _ => return Err(invalide()),
    };
    let Json(lettres) = lire(db, move |tx| {
        stockage::correspondance(tx, compte.account_id, &adresse, avant)
    })
    .await?;
    lettres.map(Json).ok_or_else(absente)
}

async fn decider(
    compte: CompteAuthentifie,
    State(db): State<Arc<Database>>,
    donnees: Result<Json<Commande>, JsonRejection>,
) -> Result<Response, ApiError> {
    let Json(mut cmd) = donnees.map_err(|_| invalide())?;
    if cmd.revision_attendue < 1 {
        return Err(invalide());
    }
    cmd.adresse = adresse(&cmd.adresse)?;
    if matches!(cmd.decision, Choix::Refuse) {
        cmd.destination = Destination::Reception;
    }
    let resultat = tokio::task::spawn_blocking(move || -> Result<_, Error> {
        Ok(stockage::appliquer(
            &mut *db.writer()?,
            compte.account_id,
            cmd,
        )?)
    })
    .await
    .map_err(unavailable)?
    .map_err(unavailable)?;
    match resultat {
        stockage::Application::Absente => Err(absente()),
        stockage::Application::Conflit(r) => Ok((StatusCode::CONFLICT, Json(r)).into_response()),
        stockage::Application::Enregistree(r) => Ok(Json(r).into_response()),
    }
}
