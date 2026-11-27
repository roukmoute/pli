mod db;
mod http;
#[cfg(test)]
mod test_support;

use std::{path::Path, sync::Arc};

use tokio::net::TcpListener;

pub type Error = Box<dyn std::error::Error + Send + Sync>;

pub async fn serve(listener: TcpListener, data_dir: &Path) -> Result<(), Error> {
    let db = Arc::new(db::Database::open(data_dir)?);
    tracing::info!(address = %listener.local_addr()?, "Serveur prêt");
    axum::serve(listener, http::router(db)).await?;
    Ok(())
}
