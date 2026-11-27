mod secret;
mod store;
#[cfg(test)]
mod tests;

use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{FromRequestParts, State, rejection::JsonRejection},
    http::{
        HeaderMap, StatusCode,
        header::{COOKIE, SET_COOKIE},
        request::Parts,
    },
    response::{IntoResponse, Response},
    routing::post,
};
use serde::Deserialize;

use crate::{
    Error, admin,
    db::Database,
    http::{ApiError, api_error},
    sync::Synchronizer,
};
use secret::Token;
pub(crate) use secret::{SessionConfig, decrypt};
pub(crate) use store::CompteAuthentifie;
use store::Mailbox;

const COOKIE_NAME: &str = "__Host-pli_session";

struct Sessions {
    db: Arc<Database>,
    config: SessionConfig,
    allow_test: bool,
    sync: Synchronizer,
}

pub(crate) fn router(
    db: Arc<Database>,
    config: SessionConfig,
    allow_test: bool,
    sync: Synchronizer,
) -> Router<Arc<Database>> {
    let open = post(open).with_state(Arc::new(Sessions {
        db,
        config,
        allow_test,
        sync,
    }));
    Router::new().route("/v1/session", open.get(current).delete(close))
}

fn invalid() -> ApiError {
    api_error(
        StatusCode::UNAUTHORIZED,
        "identifiants_invalides",
        "Adresse ou secret de messagerie incorrect.",
    )
}

fn unavailable() -> ApiError {
    api_error(
        StatusCode::SERVICE_UNAVAILABLE,
        "service_indisponible",
        "Le service de messagerie est indisponible.",
    )
}

fn expired() -> ApiError {
    api_error(
        StatusCode::UNAUTHORIZED,
        "session_expiree",
        "Veuillez vous reconnecter.",
    )
}

async fn database<T: Send + 'static>(
    db: &Arc<Database>,
    action: impl FnOnce(&Database) -> Result<T, Error> + Send + 'static,
) -> Result<T, ApiError> {
    let db = Arc::clone(db);
    tokio::task::spawn_blocking(move || action(&db))
        .await
        .map_err(|_| unavailable())?
        .map_err(|_| unavailable())
}

fn cookie_token(headers: &HeaderMap) -> Option<Token> {
    let mut cookies = headers
        .get_all(COOKIE)
        .iter()
        .filter_map(|h| h.to_str().ok())
        .flat_map(|h| h.split(';'))
        .filter_map(|pair| pair.trim().split_once('='))
        .filter(|(name, _)| *name == COOKIE_NAME);
    let (_, value) = cookies.next()?;
    if cookies.next().is_some() {
        return None;
    }
    Token::parse(value)
}

fn cookie(value: &str, max_age: u32) -> String {
    format!("{COOKIE_NAME}={value}; Path=/; Secure; HttpOnly; SameSite=Strict; Max-Age={max_age}")
}

impl FromRequestParts<Arc<Database>> for CompteAuthentifie {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, db: &Arc<Database>) -> Result<Self, ApiError> {
        let token = cookie_token(&parts.headers).ok_or_else(expired)?;
        database(db, move |db| Ok(store::current(&*db.reader()?, &token)?))
            .await?
            .ok_or_else(expired)
    }
}

#[derive(Deserialize)]
struct Credentials {
    address: String,
    password: String,
}

async fn open(
    State(sessions): State<Arc<Sessions>>,
    headers: HeaderMap,
    input: Result<Json<Credentials>, JsonRejection>,
) -> Result<Response, ApiError> {
    let Json(input) = input.map_err(|_| invalid())?;
    let mailbox = Mailbox::parse(&input.address).ok_or_else(invalid)?;
    if input.password.is_empty() {
        return Err(invalid());
    }
    let key = sessions.config.key.as_ref().ok_or_else(unavailable)?;
    let domain = mailbox.domain.clone();
    let server = database(&sessions.db, move |db| {
        admin::for_domain(&*db.reader()?, &domain)
    })
    .await?
    .ok_or_else(invalid)?;
    server
        .profile
        .validate(sessions.allow_test)
        .map_err(|_| unavailable())?;
    let imap = admin::login_mailbox(&server.profile, &mailbox.address, &input.password)
        .await
        .map_err(|failure| match failure {
            admin::LoginFailure::InvalidCredentials => invalid(),
            admin::LoginFailure::Unavailable => unavailable(),
        })?;
    let encrypted = secret::encrypt(key, server.id, &mailbox.address, input.password)
        .map_err(|_| unavailable())?;
    let address = mailbox.address.clone();
    let (account_id, has_marker) = database(&sessions.db, move |db| {
        store::attach(&mut *db.writer()?, &mailbox, &server, &encrypted)
    })
    .await?;
    sessions
        .sync
        .on_login(account_id, imap, has_marker)
        .await
        .map_err(|_| unavailable())?;
    let token = Token::generate().map_err(|_| unavailable())?;
    let ttl = sessions.config.ttl.get();
    let header = cookie(&token.encode(), ttl);
    let previous = cookie_token(&headers);
    let account = database(&sessions.db, move |db| {
        store::open(
            &mut *db.writer()?,
            account_id,
            address,
            &token,
            previous,
            ttl,
        )
    })
    .await?;
    Ok(([(SET_COOKIE, header)], Json(account)).into_response())
}

async fn current(account: CompteAuthentifie) -> Json<CompteAuthentifie> {
    Json(account)
}

async fn close(State(db): State<Arc<Database>>, headers: HeaderMap) -> Result<Response, ApiError> {
    if let Some(token) = cookie_token(&headers) {
        database(&db, move |db| Ok(store::revoke(&*db.writer()?, &token)?)).await?;
    }
    Ok((StatusCode::NO_CONTENT, [(SET_COOKIE, cookie("", 0))]).into_response())
}
