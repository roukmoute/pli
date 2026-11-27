mod db;
mod http;

use std::{env, path::PathBuf, sync::Arc};

use db::Database;
use tokio::net::TcpListener;

type Error = Box<dyn std::error::Error + Send + Sync>;

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt::init();
    let listen = env::var("WEBMAIL_LISTEN").unwrap_or_else(|_| "127.0.0.1:8080".into());
    let data_dir =
        PathBuf::from(env::var_os("WEBMAIL_DATA_DIR").unwrap_or_else(|| "./data".into()));
    let db = Arc::new(Database::open(&data_dir)?);
    let listener = TcpListener::bind(listen).await?;
    tracing::info!(address = %listener.local_addr()?, "Serveur prêt");
    axum::serve(listener, http::router(db)).await?;
    Ok(())
}
