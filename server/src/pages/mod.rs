mod storage;
pub(crate) use storage::thread as lire_resume;
#[cfg(test)]
mod tests;

use std::{env, sync::Arc};

use axum::{
    Extension, Json, Router,
    extract::{Path, Query, State, rejection::QueryRejection},
    http::StatusCode,
    routing::get,
};
use serde::{Deserialize, Serialize};

use crate::{
    Error,
    db::Database,
    http::{ApiError, api_error, unavailable},
    session::CompteAuthentifie,
};

const SIZES: std::ops::RangeInclusive<usize> = 1..=100;

/// Taille de page servie quand la requête ne précise pas `taille`.
/// Les lots d’import IMAP gardent leur propre taille.
#[derive(Clone, Copy)]
pub(crate) struct PageConfig {
    size: usize,
}

impl PageConfig {
    pub(crate) fn from_env() -> Result<Self, Error> {
        Self::parse(env::var("WEBMAIL_PAGE_SIZE").ok().as_deref())
    }

    pub(crate) fn parse(size: Option<&str>) -> Result<Self, Error> {
        let size = size.filter(|value| !value.is_empty()).unwrap_or("20");
        match size.parse() {
            Ok(size) if SIZES.contains(&size) => Ok(Self { size }),
            _ => Err("WEBMAIL_PAGE_SIZE doit être un entier de 1 à 100.".into()),
        }
    }
}

pub(crate) fn router(config: PageConfig) -> Router<Arc<Database>> {
    Router::new()
        .route("/v1/views/{vue}", get(view))
        .route("/v1/folders/{id}/messages", get(folder))
        .layer(Extension(config))
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
    /// Vrai dans la Corbeille ou les Indésirables quand le fil ne sait pas où revenir.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    sans_origine: bool,
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
    /// Nombre de fils de la liste entière, pages non chargées comprises.
    total: i64,
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
    Extension(config): Extension<PageConfig>,
    Path(name): Path<String>,
    parameters: Result<Query<Parameters>, QueryRejection>,
) -> Result<Json<Page>, ApiError> {
    if !matches!(name.as_str(), "reception" | "flux" | "justificatifs") {
        return Err(absent());
    }
    page(
        db,
        account.account_id,
        Scope::View(name),
        config,
        parameters,
    )
    .await
}

async fn folder(
    account: CompteAuthentifie,
    State(db): State<Arc<Database>>,
    Extension(config): Extension<PageConfig>,
    Path(id): Path<i64>,
    parameters: Result<Query<Parameters>, QueryRejection>,
) -> Result<Json<Page>, ApiError> {
    page(
        db,
        account.account_id,
        Scope::Folder(id),
        config,
        parameters,
    )
    .await
}

async fn page(
    db: Arc<Database>,
    account: i64,
    scope: Scope,
    config: PageConfig,
    parameters: Result<Query<Parameters>, QueryRejection>,
) -> Result<Json<Page>, ApiError> {
    let Query(parameters) = parameters.map_err(|_| invalid())?;
    let size = parameters.taille.unwrap_or(config.size);
    if !SIZES.contains(&size) {
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
