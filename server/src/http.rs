use std::{fmt::Display, path::PathBuf, sync::Arc};

use axum::{
    Json, Router,
    extract::{Request, State},
    http::{HeaderValue, StatusCode, header::CACHE_CONTROL},
    middleware::{self, Next},
    response::Response,
    routing::{get, put},
};
use serde::Serialize;
use tower_http::{
    services::{ServeDir, ServeFile},
    set_header::SetResponseHeaderLayer,
};

use crate::{AdminConfig, Error, admin, db::Database, session, sync};

pub fn router(
    db: Arc<Database>,
    config: AdminConfig,
    sessions: session::SessionConfig,
) -> Result<Router, Error> {
    let sync = sync::Synchronizer::start(db.clone(), sessions.key.clone(), config.allow_test)?;
    let lecteur = sync.lecteur();
    let api = Router::new()
        .merge(crate::lecture::router(lecteur.clone()))
        .merge(crate::operations::router(lecteur))
        .merge(session::router(
            db.clone(),
            sessions,
            config.allow_test,
            sync,
        ))
        .merge(crate::pages::router(crate::pages::PageConfig::from_env()?))
        .merge(crate::tri::router())
        .route("/v1/folders", get(sync::folders))
        .route("/v1/folders/archive", put(sync::choisir_archives))
        .nest("/admin/v1", admin::router(db.clone(), config))
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
    let root = std::env::var_os("WEBMAIL_WEB_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("web/dist"));
    let mut files = Router::new();
    for path in [
        "/admin",
        "/admin/",
        "/reception",
        "/flux",
        "/justificatifs",
        "/nouveaux-expediteurs",
        "/historique-du-tri",
        "/dossier/{id}",
        "/fil/{fil}",
        "/fil/{fil}/message/{id}",
        "/ecrire",
    ] {
        files = files.route_service(path, ServeFile::new(root.join("index.html")));
    }
    let files = files
        .fallback_service(ServeDir::new(root))
        .layer(middleware::from_fn(cache_static));
    Ok(Router::new().nest("/api", api).fallback_service(files))
}

#[derive(Serialize)]
struct Health {
    status: &'static str,
}

#[derive(Serialize)]
pub(crate) struct ErrorBody {
    code: &'static str,
    message: &'static str,
}

pub(crate) type ApiError = (StatusCode, Json<ErrorBody>);

pub(crate) fn api_error(status: StatusCode, code: &'static str, message: &'static str) -> ApiError {
    (status, Json(ErrorBody { code, message }))
}

pub(crate) fn unavailable(error: impl Display) -> ApiError {
    tracing::error!(%error, "Accès SQLite indisponible");
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

#[cfg(test)]
mod tests {
    use axum::{body::Body, http::header::CONTENT_TYPE};
    use tower::ServiceExt;

    use super::*;
    use crate::test_support::TestApp;

    #[tokio::test]
    async fn health_returns_ok() -> Result<(), Error> {
        let app = TestApp::new()?;
        let request = Request::get("/api/v1/health").body(Body::empty())?;
        let response = app.router().oneshot(request).await?;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()[CONTENT_TYPE], "application/json");
        Ok(())
    }

    #[tokio::test]
    async fn caches_only_successful_or_revalidated_fingerprinted_assets() -> Result<(), Error> {
        let asset = "/assets/app-Ab12_3-x.js";
        let immutable = "public, max-age=31536000, immutable";
        for (path, status, policy) in [
            (asset, StatusCode::OK, immutable),
            (asset, StatusCode::NOT_MODIFIED, immutable),
            (asset, StatusCode::NOT_FOUND, "no-cache"),
            ("/app-Ab12_3-x.js", StatusCode::OK, "no-cache"),
            ("/assets/app.js", StatusCode::OK, "no-cache"),
            ("/assets/app-Ab12.3-x.js", StatusCode::OK, "no-cache"),
        ] {
            let files = Router::new()
                .fallback(move || async move { status })
                .layer(middleware::from_fn(cache_static));
            let response = files
                .oneshot(Request::get(path).body(Body::empty())?)
                .await?;
            assert_eq!(response.headers()[CACHE_CONTROL], policy, "{status} {path}");
        }
        Ok(())
    }
}
