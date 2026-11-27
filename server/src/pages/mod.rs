mod storage;
pub(crate) use storage::thread as lire_resume;
#[cfg(test)]
mod tests;

use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, Query, State, rejection::QueryRejection},
    http::StatusCode,
    routing::get,
};
use serde::{Deserialize, Serialize};

use crate::{
    db::Database,
    http::{ApiError, api_error, unavailable},
    session::CompteAuthentifie,
};

pub(crate) fn router() -> Router<Arc<Database>> {
    Router::new()
        .route("/v1/views/{vue}", get(view))
        .route("/v1/folders/{id}/messages", get(folder))
}

#[derive(Deserialize, Default)]
struct Parameters {
    taille: Option<usize>,
    curseur: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct Cursor {
    compte: i64,
    portee: String,
    date: i64,
    fil: String,
}

impl Cursor {
    fn decode(value: &str, account: i64, scope: &str) -> Option<Self> {
        let cursor: Self = serde_json::from_slice(&hex::decode(value).ok()?).ok()?;
        (cursor.compte == account && cursor.portee == scope).then_some(cursor)
    }
}

enum Scope {
    View(String),
    Folder(i64),
}

impl Scope {
    fn key(&self) -> String {
        match self {
            Self::View(v) => v.clone(),
            Self::Folder(id) => format!("dossier/{id}"),
        }
    }
}

#[derive(Serialize)]
struct Sender {
    nom: Option<String>,
    adresse: Option<String>,
}

#[derive(Serialize)]
pub(crate) struct Thread {
    id: String,
    objet: String,
    expediteur: Sender,
    extrait: String,
    date: i64,
    nombre_messages: i64,
    non_lu: bool,
    etoile: bool,
    piece_jointe: bool,
    repondu: bool,
    transfere: bool,
}

#[derive(Serialize)]
struct Coverage {
    import_en_cours: bool,
    indexes: i64,
    attendus: Option<i64>,
    proportion_indexee: Option<f64>,
}

#[derive(Serialize, Default)]
struct Counts {
    reception: i64,
    flux: i64,
    justificatifs: i64,
}

#[derive(Serialize)]
struct Page {
    compte: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    vue: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dossier: Option<i64>,
    nouveaux: Vec<Thread>,
    deja_vus: Vec<Thread>,
    curseur_suivant: Option<String>,
    revision: i64,
    couverture: Coverage,
    compteurs: Counts,
    expediteurs_en_attente: i64,
}

fn invalid() -> ApiError {
    api_error(
        StatusCode::BAD_REQUEST,
        "pagination_invalide",
        "Taille ou curseur de page invalide.",
    )
}

fn absent() -> ApiError {
    api_error(
        StatusCode::NOT_FOUND,
        "liste_introuvable",
        "Liste de courrier introuvable.",
    )
}

async fn view(
    account: CompteAuthentifie,
    State(db): State<Arc<Database>>,
    Path(name): Path<String>,
    parameters: Result<Query<Parameters>, QueryRejection>,
) -> Result<Json<Page>, ApiError> {
    if !matches!(name.as_str(), "reception" | "flux" | "justificatifs") {
        return Err(absent());
    }
    page(db, account.account_id, Scope::View(name), parameters).await
}

async fn folder(
    account: CompteAuthentifie,
    State(db): State<Arc<Database>>,
    Path(id): Path<i64>,
    parameters: Result<Query<Parameters>, QueryRejection>,
) -> Result<Json<Page>, ApiError> {
    page(db, account.account_id, Scope::Folder(id), parameters).await
}

async fn page(
    db: Arc<Database>,
    account: i64,
    scope: Scope,
    parameters: Result<Query<Parameters>, QueryRejection>,
) -> Result<Json<Page>, ApiError> {
    let Query(parameters) = parameters.map_err(|_| invalid())?;
    let size = parameters.taille.unwrap_or(20);
    if !(1..=100).contains(&size) {
        return Err(invalid());
    }
    let cursor = parameters
        .curseur
        .map(|raw| Cursor::decode(&raw, account, &scope.key()).ok_or_else(invalid))
        .transpose()?;
    tokio::task::spawn_blocking(move || -> Result<_, crate::Error> {
        let mut reader = db.reader()?;
        Ok(storage::read(
            &mut reader,
            account,
            scope,
            size,
            cursor.as_ref(),
        )?)
    })
    .await
    .map_err(unavailable)?
    .map_err(unavailable)?
    .map(Json)
    .ok_or_else(absent)
}
