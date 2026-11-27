use std::sync::Arc;

use axum::Router;
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
        http::router(Arc::clone(&self.db), Default::default())
    }
}
