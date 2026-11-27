use std::{
    fs,
    path::Path,
    sync::{Mutex, MutexGuard},
};

use include_dir::{Dir, include_dir};
use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{Connection, OpenFlags};
use rusqlite_migration::Migrations;

use crate::Error;

static MIGRATIONS: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/migrations");

pub struct Database {
    writer: Mutex<Connection>,
    readers: Pool<SqliteConnectionManager>,
}

impl Database {
    pub fn open(data_dir: &Path) -> Result<Self, Error> {
        fs::create_dir_all(data_dir)?;
        let path = data_dir.join("webmail.db");
        let writer = Connection::open(&path)?;
        let mode: String = writer.query_row("PRAGMA journal_mode = WAL", [], |row| row.get(0))?;
        if mode != "wal" {
            return Err(format!("SQLite a conservé le mode {mode} au lieu de WAL").into());
        }
        writer.pragma_update(None, "foreign_keys", true)?;

        let readers = SqliteConnectionManager::file(&path)
            .with_flags(OpenFlags::SQLITE_OPEN_READ_ONLY)
            .with_init(|conn| conn.pragma_update(None, "foreign_keys", true));
        let database = Self {
            writer: Mutex::new(writer),
            readers: Pool::builder().max_size(4).build(readers)?,
        };
        Migrations::from_directory(&MIGRATIONS)?.to_latest(&mut *database.writer()?)?;
        Ok(database)
    }

    pub fn writer(&self) -> Result<MutexGuard<'_, Connection>, Error> {
        self.writer
            .lock()
            .map_err(|_| "Connexion d’écriture interrompue".into())
    }

    pub fn reader(&self) -> Result<PooledConnection<SqliteConnectionManager>, r2d2::Error> {
        self.readers.get()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TestApp;

    fn assert_current_schema(db: &Database) -> Result<(), Error> {
        let reader = db.reader()?;
        let version: u32 = reader.query_row("PRAGMA user_version", [], |row| row.get(0))?;
        assert_eq!(version, u32::try_from(MIGRATIONS.dirs().count())?);
        let id: u32 = reader.query_row("PRAGMA application_id", [], |row| row.get(0))?;
        assert_eq!(id, 0x506C6900);
        Ok(())
    }

    #[test]
    fn readers_keep_the_committed_schema_during_a_write() -> Result<(), Error> {
        let app = TestApp::new()?;
        let database = &app.db;
        let mut writer = database.writer()?;
        assert!(writer.query_row("PRAGMA foreign_keys", [], |row| row.get::<_, bool>(0))?);
        let transaction = writer.transaction()?;
        transaction.execute_batch("CREATE TABLE pending (id INTEGER PRIMARY KEY)")?;

        let first = database.reader()?;
        let second = database.reader()?;
        for reader in [&first, &second] {
            assert!(reader.query_row("PRAGMA foreign_keys", [], |row| row.get::<_, bool>(0))?);
            let tables: i32 = reader.query_row(
                "SELECT count(*) FROM sqlite_schema WHERE name = 'pending'",
                [],
                |row| row.get(0),
            )?;
            assert_eq!(tables, 0);
        }
        transaction.commit()?;
        let error = first
            .execute("INSERT INTO pending VALUES (1)", [])
            .unwrap_err();
        assert_eq!(
            error.sqlite_error_code(),
            Some(rusqlite::ErrorCode::ReadOnly)
        );
        Ok(())
    }

    #[test]
    fn migrations_apply_to_an_empty_database_and_survive_restart() -> Result<(), Error> {
        let app = TestApp::new()?;
        assert_current_schema(&app.db)?;
        app.db.writer()?.execute_batch(
            "CREATE TABLE restart_probe (id INTEGER PRIMARY KEY);
             INSERT INTO restart_probe VALUES (42);",
        )?;

        let restarted = app.reopen()?;
        assert_current_schema(&restarted.db)?;
        let reader = restarted.db.reader()?;
        let saved: i64 = reader.query_row("SELECT id FROM restart_probe", [], |row| row.get(0))?;
        assert_eq!(saved, 42);
        Ok(())
    }

    #[test]
    fn rejects_a_newer_schema() -> Result<(), Error> {
        let app = TestApp::new()?;
        let future_version = u32::try_from(MIGRATIONS.dirs().count())? + 1;
        app.db
            .writer()?
            .pragma_update(None, "user_version", future_version)?;
        assert!(app.reopen().is_err());
        Ok(())
    }

    #[test]
    fn uses_wal() -> Result<(), Error> {
        let app = TestApp::new()?;
        let mode: String = app
            .db
            .reader()?
            .query_row("PRAGMA journal_mode", [], |row| row.get(0))?;
        assert_eq!(mode, "wal");
        Ok(())
    }

    #[test]
    fn can_create_and_query_an_fts5_table() -> Result<(), Error> {
        let app = TestApp::new()?;
        let writer = app.db.writer()?;
        writer.execute_batch(
            "CREATE VIRTUAL TABLE search_probe USING fts5(subject);
             INSERT INTO search_probe VALUES ('message de test');",
        )?;
        let subject: String = writer.query_row(
            "SELECT subject FROM search_probe WHERE search_probe MATCH 'message'",
            [],
            |row| row.get(0),
        )?;
        assert_eq!(subject, "message de test");
        Ok(())
    }
}
