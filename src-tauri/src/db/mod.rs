pub mod schema;

use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::Mutex;

pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    pub fn open(path: &PathBuf) -> Result<Self, rusqlite::Error> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }

        let conn = Connection::open(path)?;

        conn.execute_batch("PRAGMA journal_mode=WAL;")?;
        conn.execute_batch("PRAGMA foreign_keys=ON;")?;
        conn.execute_batch("PRAGMA busy_timeout=5000;")?;

        schema::run_migrations(&conn)?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn conn(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn.lock().expect("database mutex poisoned")
    }
}

pub fn data_dir() -> PathBuf {
    let dirs = directories::ProjectDirs::from("dev", "lumi", "Lumi")
        .expect("failed to resolve data directory");
    dirs.data_dir().to_path_buf()
}

pub fn db_path() -> PathBuf {
    data_dir().join("lumi.db")
}
