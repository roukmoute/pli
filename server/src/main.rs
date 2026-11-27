use std::{env, path::PathBuf};

use tokio::net::TcpListener;
use webmail_server::Error;

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Les traces de protocole des bibliothèques peuvent inclure les commandes de connexion.
    use tracing_subscriber::{filter::Targets, layer::SubscriberExt, util::SubscriberInitExt};
    tracing_subscriber::registry()
        .with(Targets::new().with_target("webmail_server", tracing::Level::INFO))
        .with(tracing_subscriber::fmt::layer())
        .init();
    let listen = env::var("WEBMAIL_LISTEN").unwrap_or_else(|_| "127.0.0.1:8080".into());
    let data_dir =
        PathBuf::from(env::var_os("WEBMAIL_DATA_DIR").unwrap_or_else(|| "./data".into()));
    webmail_server::serve(TcpListener::bind(listen).await?, &data_dir).await
}
