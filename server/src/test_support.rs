use std::sync::Arc;

use axum::Router;
use tokio_rustls::{
    TlsAcceptor,
    rustls::{ServerConfig, pki_types::PrivatePkcs8KeyDer},
};

use tempfile::TempDir;

use crate::{Error, db::Database, http};

pub(crate) struct TestApp {
    pub db: Arc<Database>,
    data_dir: TempDir,
}

impl TestApp {
    pub fn new() -> Result<Self, Error> {
        Self::open(tempfile::tempdir()?)
    }

    fn open(data_dir: TempDir) -> Result<Self, Error> {
        let db = Arc::new(Database::open(data_dir.path())?);
        Ok(Self { db, data_dir })
    }

    pub fn reopen(self) -> Result<Self, Error> {
        drop(self.db);
        Self::open(self.data_dir)
    }

    pub fn router(&self) -> Router {
        http::router(
            Arc::clone(&self.db),
            Default::default(),
            crate::session::SessionConfig::parse(None, "60").unwrap(),
        )
        .unwrap()
    }
}

pub(crate) fn tls_acceptor() -> TlsAcceptor {
    let cert = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
    let key = PrivatePkcs8KeyDer::from(cert.signing_key.serialize_der());
    let config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![cert.cert.der().clone()], key.into())
        .unwrap();
    TlsAcceptor::from(Arc::new(config))
}
