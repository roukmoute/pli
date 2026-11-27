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

    #[test]
    fn readers_keep_the_committed_schema_during_a_write() -> Result<(), Error> {
        let dir = tempfile::tempdir()?;
        let database = Database::open(dir.path())?;
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
    fn migrations_survive_restart_and_reject_a_newer_schema() -> Result<(), Error> {
        let dir = tempfile::tempdir()?;
        drop(Database::open(dir.path())?);
        let database = Database::open(dir.path())?;
        let conn = database.reader()?;
        let mode: String = conn.query_row("PRAGMA journal_mode", [], |row| row.get(0))?;
        assert_eq!(mode, "wal");
        let version: u32 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
        assert!(version >= 1);
        let id: u32 = conn.query_row("PRAGMA application_id", [], |row| row.get(0))?;
        assert_eq!(id, 0x506C6900);
        drop(conn);
        database
            .writer()?
            .pragma_update(None, "user_version", version + 1)?;
        drop(database);
        assert!(Database::open(dir.path()).is_err());
        Ok(())
    }
}
