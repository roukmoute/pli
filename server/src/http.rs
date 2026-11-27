use std::{fmt::Display, sync::Arc};

use axum::{
    Json, Router,
    extract::{Request, State},
    http::{HeaderValue, StatusCode, header::CACHE_CONTROL},
    middleware::{self, Next},
    response::Response,
    routing::get,
};
use serde::Serialize;
use tower_http::{services::ServeDir, set_header::SetResponseHeaderLayer};

use crate::{Error, db::Database};

pub fn router(db: Arc<Database>) -> Router {
    let api = Router::new()
        .route("/v1/health", get(health))
        .fallback(async || api_error(StatusCode::NOT_FOUND, "not_found", "Route API inconnue."))
        .method_not_allowed_fallback(async || {
            api_error(
                StatusCode::METHOD_NOT_ALLOWED,
                "method_not_allowed",
                "Méthode non autorisée.",
            )
        })
        .layer(SetResponseHeaderLayer::overriding(
            CACHE_CONTROL,
            HeaderValue::from_static("no-store"),
        ))
        .with_state(db);
    let files = Router::new()
        .fallback_service(ServeDir::new("web/dist"))
        .layer(middleware::from_fn(cache_static));
    Router::new().nest("/api", api).fallback_service(files)
}

#[derive(Serialize)]
struct Health {
    status: &'static str,
}

#[derive(Serialize)]
struct ErrorBody {
    code: &'static str,
    message: &'static str,
}

type ApiError = (StatusCode, Json<ErrorBody>);

fn api_error(status: StatusCode, code: &'static str, message: &'static str) -> ApiError {
    (status, Json(ErrorBody { code, message }))
}

fn unavailable(error: impl Display) -> ApiError {
    tracing::error!(%error, "Échec de la lecture SQLite");
    api_error(
        StatusCode::SERVICE_UNAVAILABLE,
        "database_unavailable",
        "Base de données indisponible.",
    )
}

async fn health(State(db): State<Arc<Database>>) -> Result<Json<Health>, ApiError> {
    tokio::task::spawn_blocking(move || -> Result<(), Error> {
        db.reader()?
            .query_row("PRAGMA user_version", [], |_| Ok(()))?;
        Ok(())
    })
    .await
    .map_err(unavailable)?
    .map_err(unavailable)?;
    Ok(Json(Health { status: "ok" }))
}

async fn cache_static(request: Request, next: Next) -> Response {
    let fingerprinted = has_vite_fingerprint(request.uri().path());
    let mut response = next.run(request).await;
    let status = response.status();
    let policy = if fingerprinted && (status.is_success() || status == StatusCode::NOT_MODIFIED) {
        "public, max-age=31536000, immutable"
    } else {
        "no-cache"
    };
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static(policy));
    response
}

fn has_vite_fingerprint(path: &str) -> bool {
    let Some((name, _)) = path
        .strip_prefix("/assets/")
        .and_then(|file| file.rsplit_once('.'))
    else {
        return false;
    };
    // Vite produit [name]-[hash].[ext] avec huit caractères base64 URL.
    let Some([b'-', hash @ ..]) = name.as_bytes().last_chunk::<9>() else {
        return false;
    };
    hash.iter()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}
