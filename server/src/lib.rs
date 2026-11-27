mod admin;
mod db;
mod http;
mod pages;
mod session;
mod sync;
#[cfg(test)]
mod test_support;

use std::{path::Path, sync::Arc};

pub use admin::AdminConfig;
use tokio::net::TcpListener;

pub type Error = Box<dyn std::error::Error + Send + Sync>;

pub fn init_logging(
    writer: impl for<'a> tracing_subscriber::fmt::MakeWriter<'a> + Send + Sync + 'static,
) {
    use tracing_subscriber::{filter::Targets, layer::SubscriberExt, util::SubscriberInitExt};
    // Les traces de protocole des bibliothèques peuvent inclure les commandes de connexion.
    tracing_subscriber::registry()
        .with(Targets::new().with_target("webmail_server", tracing::Level::INFO))
        .with(tracing_subscriber::fmt::layer().with_writer(writer))
        .init();
}

pub async fn serve(listener: TcpListener, data_dir: &Path) -> Result<(), Error> {
    serve_with_admin(listener, data_dir, AdminConfig::from_env()).await
}

pub async fn serve_with_admin(
    listener: TcpListener,
    data_dir: &Path,
    admin: AdminConfig,
) -> Result<(), Error> {
    let sessions = session::SessionConfig::from_env()?;
    let db = Arc::new(db::Database::open(data_dir)?);
    let router = http::router(db, admin, sessions)?;
    tracing::info!(address = %listener.local_addr()?, "Serveur prêt");
    axum::serve(listener, router).await?;
    Ok(())
}
