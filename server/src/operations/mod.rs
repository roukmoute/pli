mod deplacement;
mod stockage;
#[cfg(test)]
mod tests;

use std::sync::Arc;

use axum::{
    Extension, Json, Router,
    extract::{Path, State, rejection::JsonRejection},
    http::StatusCode,
    routing::{get, post},
};

use crate::{
    db::Database,
    http::{ApiError, api_error},
    lecture::sqlite,
    session::CompteAuthentifie,
    sync::Lecteur,
};
pub(crate) use deplacement::tenter;
use stockage::Empechement;
pub(crate) use stockage::{Action, Depot, Lot, Sort, comptes_en_attente, conclure, prochain};

pub(crate) fn router(lecteur: Lecteur) -> Router<Arc<Database>> {
    Router::new()
        .route("/v1/operations", post(creer))
        .route("/v1/operations/{id}", get(consulter))
        .layer(Extension(lecteur))
}

fn invalide(statut: StatusCode) -> ApiError {
    api_error(statut, "operation_invalide", "Opération invalide.")
}

// Le client choisit l’identifiant : un UUID sous sa forme canonique.
fn identifiant(id: &str) -> bool {
    id.len() == 36
        && id.bytes().enumerate().all(|(rang, octet)| match rang {
            8 | 13 | 18 | 23 => octet == b'-',
            _ => octet.is_ascii_hexdigit(),
        })
}

async fn creer(
    compte: CompteAuthentifie,
    State(db): State<Arc<Database>>,
    Extension(lecteur): Extension<Lecteur>,
    entree: Result<Json<stockage::Demande>, JsonRejection>,
) -> Result<Json<stockage::Operation>, ApiError> {
    let Json(demande) = entree.map_err(|e| invalide(e.status()))?;
    if !identifiant(&demande.id) || !(1..=100).contains(&demande.cibles.len()) {
        return Err(invalide(StatusCode::BAD_REQUEST));
    }
    let compte = compte.account_id;
    let operation = sqlite(db, move |db| stockage::creer(db, compte, &demande))
        .await?
        .map_err(|empechement| {
            let (code, message) = match empechement {
                Empechement::Archives => (
                    "archives_inconnues",
                    "Aucun dossier ne reçoit les archives de ce compte.",
                ),
                Empechement::Corbeille => (
                    "corbeille_inconnue",
                    "La messagerie de ce compte n’annonce aucune corbeille.",
                ),
                Empechement::Origine => (
                    "origine_inconnue",
                    "Le dossier d’origine de ce courrier est inconnu.",
                ),
                Empechement::Destination => (
                    "destination_inconnue",
                    "Ce dossier n’existe plus ou ne reçoit pas de courrier.",
                ),
            };
            api_error(StatusCode::CONFLICT, code, message)
        })?;
    // La réponse part sans attendre la messagerie.
    lecteur.appliquer(compte);
    Ok(Json(operation))
}

async fn consulter(
    compte: CompteAuthentifie,
    State(db): State<Arc<Database>>,
    Path(id): Path<String>,
) -> Result<Json<stockage::Operation>, ApiError> {
    sqlite(db, move |db| {
        Ok(stockage::lire(&*db.reader()?, compte.account_id, &id)?)
    })
    .await?
    .map(Json)
    .ok_or_else(|| {
        api_error(
            StatusCode::NOT_FOUND,
            "operation_introuvable",
            "Cette opération est introuvable.",
        )
    })
}
