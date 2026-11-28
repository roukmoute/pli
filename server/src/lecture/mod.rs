mod indicateurs;
mod rendu;
mod stockage;

use std::sync::Arc;

use axum::{
    Extension, Json, Router,
    extract::{Path, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::Serialize;

use crate::{
    Error,
    db::Database,
    http::{ApiError, api_error, unavailable},
    session::CompteAuthentifie,
    sync::Lecteur,
};
pub(crate) use indicateurs::{LU, enregistrer};
pub(crate) use stockage::Cle;

pub(crate) fn router(lecteur: Lecteur) -> Router<Arc<Database>> {
    Router::new()
        .route("/v1/threads/{id}", get(fil))
        .route("/v1/messages/{id}", get(contenu))
        .route("/v1/messages/{id}/open", post(ouvrir))
        .route("/v1/messages/{id}/images/{partie}", get(image))
        .layer(Extension(lecteur))
}

fn absent() -> ApiError {
    api_error(
        StatusCode::NOT_FOUND,
        "courrier_introuvable",
        "Ce courrier est introuvable.",
    )
}

pub(crate) async fn sqlite<T: Send + 'static>(
    db: Arc<Database>,
    action: impl FnOnce(&Database) -> Result<T, Error> + Send + 'static,
) -> Result<T, ApiError> {
    tokio::task::spawn_blocking(move || action(&db))
        .await
        .map_err(unavailable)?
        .map_err(unavailable)
}

async fn fil(
    account: CompteAuthentifie,
    State(db): State<Arc<Database>>,
    Path(id): Path<String>,
) -> Result<Json<Vec<stockage::Entete>>, ApiError> {
    let messages = sqlite(db, move |db| stockage::fil(db, account.account_id, id)).await?;
    if messages.is_empty() {
        return Err(absent());
    }
    Ok(Json(messages))
}

#[derive(Serialize)]
#[serde(tag = "etat", rename_all = "lowercase")]
enum Corps {
    Disponible {
        version: u32,
        contenu: rendu::Contenu,
    },
    Indisponible,
}

impl Corps {
    fn disponible(contenu: rendu::Contenu) -> Json<Self> {
        Json(Self::Disponible {
            version: rendu::VERSION,
            contenu,
        })
    }
}

async fn contenu(
    account: CompteAuthentifie,
    State(db): State<Arc<Database>>,
    Extension(lecteur): Extension<Lecteur>,
    Path(id): Path<String>,
) -> Result<Json<Corps>, ApiError> {
    let compte = account.account_id;
    let cle = Cle::lire(&id).ok_or_else(absent)?;
    let cible = sqlite(db.clone(), move |db| stockage::cible(db, compte, cle))
        .await?
        .ok_or_else(absent)?;
    if let Some(contenu) = cible.contenu {
        return Ok(Corps::disponible(contenu));
    }
    let bytes = match lecteur.contenu(compte, cle, &cible.chemin).await {
        Ok(bytes) => bytes.ok_or_else(absent)?,
        Err(_) => return Ok(Json(Corps::Indisponible)),
    };
    sqlite(db, move |db| {
        let (contenu, images) = rendu::analyser(&bytes)?;
        Ok(stockage::conserver(db, compte, cle, &contenu, &images)?.then_some(contenu))
    })
    .await?
    .map(Corps::disponible)
    .ok_or_else(absent)
}

async fn ouvrir(
    account: CompteAuthentifie,
    State(db): State<Arc<Database>>,
    Extension(lecteur): Extension<Lecteur>,
    Path(id): Path<String>,
) -> Result<Json<stockage::Lecture>, ApiError> {
    let compte = account.account_id;
    let cle = Cle::lire(&id).ok_or_else(absent)?;
    let _ecriture = lecteur.ecriture(compte).await;
    let chemin = sqlite(db.clone(), move |db| {
        Ok(stockage::chemin(&*db.reader()?, compte, cle)?)
    })
    .await?
    .ok_or_else(absent)?;
    let non_confirmee = |_| {
        api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "lecture_non_confirmee",
            "La messagerie n’a pas confirmé la lecture.",
        )
    };
    // Les opérations plus anciennes passent d’abord : la lecture reste la dernière intention.
    lecteur.rattraper(compte).await.map_err(non_confirmee)?;
    let confirmations = lecteur
        .marquer(compte, cle.generation, &chemin, &[cle.uid], LU, true)
        .await
        .map_err(non_confirmee)?;
    if confirmations.get(&cle.uid) != Some(&true) {
        return Err(absent());
    }
    sqlite(db, move |db| stockage::confirmer(db, compte, cle))
        .await?
        .map(Json)
        .ok_or_else(absent)
}

async fn image(
    account: CompteAuthentifie,
    State(db): State<Arc<Database>>,
    Path((id, partie)): Path<(String, u32)>,
) -> Result<Response, ApiError> {
    let cle = Cle::lire(&id).ok_or_else(absent)?;
    let (mime, octets) = sqlite(db, move |db| {
        stockage::image(db, account.account_id, cle, partie)
    })
    .await?
    .ok_or_else(absent)?;
    Ok((
        [
            (header::CONTENT_TYPE, mime.as_str()),
            (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
            (
                header::CONTENT_SECURITY_POLICY,
                "default-src 'none'; sandbox",
            ),
        ],
        octets,
    )
        .into_response())
}
