mod diagnostic;
mod profiles;
#[cfg(test)]
mod tests;

pub(crate) use diagnostic::{ImapSession, LoginFailure, login_mailbox};
pub(crate) use profiles::{Server, for_domain};

use std::{env, sync::Arc};

use axum::{
    Json, Router,
    extract::{Path, Request, State, rejection::JsonRejection},
    http::{
        StatusCode,
        header::{AUTHORIZATION, WWW_AUTHENTICATE},
    },
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post, put},
};
use serde::{Deserialize, Serialize};
use subtle::ConstantTimeEq;

use crate::{
    Error,
    db::Database,
    http::{ApiError, api_error, unavailable},
};
use diagnostic::{Credentials, Diagnostic, diagnose};
use profiles::Profile;

#[derive(Default)]
pub struct AdminConfig {
    token: Option<String>,
    pub(crate) allow_test: bool,
}

impl AdminConfig {
    pub fn new(token: Option<String>, allow_test: bool) -> Self {
        Self {
            token: token.filter(|t| !t.trim().is_empty()),
            allow_test,
        }
    }

    pub(crate) fn from_env() -> Self {
        Self::new(
            env::var("WEBMAIL_ADMIN_TOKEN").ok(),
            env::var("WEBMAIL_ALLOW_TEST_PROFILE").as_deref() == Ok("1"),
        )
    }
}

#[derive(Clone)]
struct Admin {
    db: Arc<Database>,
    config: Arc<AdminConfig>,
}

pub(super) fn router(db: Arc<Database>, config: AdminConfig) -> Router<Arc<Database>> {
    let admin = Admin {
        db,
        config: Arc::new(config),
    };
    Router::new()
        .route("/servers", get(list).post(create))
        .route("/servers/test", post(test))
        .route("/servers/{id}", put(update))
        .fallback(async || {
            api_error(
                StatusCode::NOT_FOUND,
                "not_found",
                "Route d’administration inconnue.",
            )
        })
        .layer(middleware::from_fn_with_state(
            admin.config.clone(),
            authorize,
        ))
        .with_state(admin)
}

async fn authorize(
    State(config): State<Arc<AdminConfig>>,
    request: Request,
    next: Next,
) -> Response {
    let credentials = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split_once(' '));
    if let (Some(expected), Some((scheme, token))) = (&config.token, credentials)
        && scheme.eq_ignore_ascii_case("Bearer")
        && bool::from(expected.as_bytes().ct_eq(token.as_bytes()))
    {
        return next.run(request).await;
    }
    let mut response = api_error(
        StatusCode::UNAUTHORIZED,
        "unauthorized",
        "Jeton d’administration requis ou incorrect.",
    )
    .into_response();
    response.headers_mut().insert(
        WWW_AUTHENTICATE,
        axum::http::HeaderValue::from_static("Bearer"),
    );
    response
}

#[derive(Serialize)]
struct Catalog {
    servers: Vec<Server>,
    presets: Vec<Profile>,
}

async fn list(State(admin): State<Admin>) -> Result<Json<Catalog>, ApiError> {
    let servers = tokio::task::spawn_blocking(move || -> Result<_, Error> {
        let reader = admin.db.reader()?;
        Ok(profiles::list(&reader)?)
    })
    .await
    .map_err(unavailable)?
    .map_err(unavailable)?;
    Ok(Json(Catalog {
        servers,
        presets: Profile::presets(admin.config.allow_test),
    }))
}

#[derive(Deserialize)]
struct Candidate {
    profile: Profile,
    credentials: Credentials,
}

type Input = Result<Json<Candidate>, JsonRejection>;

impl Candidate {
    fn read(input: Input, allow_test: bool) -> Result<Self, ApiError> {
        let invalid =
            |message| api_error(StatusCode::UNPROCESSABLE_ENTITY, "invalid_profile", message);
        let Json(candidate) =
            input.map_err(|_| invalid("Profil ou identifiants de test mal formés."))?;
        candidate.profile.validate(allow_test).map_err(invalid)?;
        if candidate.credentials.username.trim().is_empty()
            || candidate.credentials.password.is_empty()
        {
            return Err(invalid("Les identifiants du test sont requis."));
        }
        Ok(candidate)
    }

    async fn diagnostic(&self) -> Diagnostic {
        diagnose(&self.profile, &self.credentials).await
    }
}

async fn test(State(admin): State<Admin>, input: Input) -> Result<Json<Diagnostic>, ApiError> {
    Ok(Json(
        Candidate::read(input, admin.config.allow_test)?
            .diagnostic()
            .await,
    ))
}

async fn create(State(admin): State<Admin>, input: Input) -> Result<Response, ApiError> {
    persist(admin, None, input).await
}

async fn update(
    State(admin): State<Admin>,
    Path(id): Path<i64>,
    input: Input,
) -> Result<Response, ApiError> {
    persist(admin, Some(id), input).await
}

#[derive(Serialize)]
struct SaveResult {
    diagnostic: Diagnostic,
    server: Option<Server>,
}

async fn persist(admin: Admin, id: Option<i64>, input: Input) -> Result<Response, ApiError> {
    let candidate = Candidate::read(input, admin.config.allow_test)?;
    let diagnostic = candidate.diagnostic().await;
    if !diagnostic.passed() {
        return Ok((
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(serde_json::json!({
                "code": "diagnostic_failed",
                "message": "Le diagnostic a échoué.",
                "diagnostic": diagnostic,
                "server": null,
            })),
        )
            .into_response());
    }
    let server = tokio::task::spawn_blocking(move || {
        let writer = admin.db.writer()?;
        profiles::save(&writer, id, candidate.profile)
    })
    .await
    .map_err(unavailable)?
    .map_err(unavailable)?
    .ok_or_else(|| api_error(StatusCode::NOT_FOUND, "not_found", "Profil introuvable."))?;
    let status = if id.is_some() {
        StatusCode::OK
    } else {
        StatusCode::CREATED
    };
    Ok((
        status,
        Json(SaveResult {
            diagnostic,
            server: Some(server),
        }),
    )
        .into_response())
}
