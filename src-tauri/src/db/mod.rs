pub mod schema;

use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::Connection;
use std::path::PathBuf;

/// Connection pool wrapper. Each `db.conn()` call pulls a pooled `Connection`
/// (deref's to `rusqlite::Connection`) and returns it to the pool on drop.
/// Replaces the previous single-connection `Mutex<Connection>` which
/// serialized every DB-touching Tauri command behind multi-second whisper
/// transcribes and LLM tool calls.
pub struct Database {
    pool: Pool<SqliteConnectionManager>,
}

/// A pooled connection. Callers use this the same way they used the old
/// `MutexGuard<Connection>` — it derefs to `rusqlite::Connection`.
pub type DbConn = r2d2::PooledConnection<SqliteConnectionManager>;

impl Database {
    pub fn open(path: &PathBuf) -> Result<Self, rusqlite::Error> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }

        // Every pooled connection gets the same pragmas + schema check.
        let manager = SqliteConnectionManager::file(path).with_init(|c: &mut Connection| {
            c.execute_batch(
                "PRAGMA journal_mode=WAL;
                 PRAGMA foreign_keys=ON;
                 PRAGMA busy_timeout=5000;",
            )
        });

        // Small pool — Tauri apps rarely run more than a few concurrent
        // DB-touching commands. Oversizing wastes file handles.
        let pool = Pool::builder()
            .max_size(8)
            .build(manager)
            .map_err(|e| rusqlite::Error::InvalidPath(format!("pool init: {}", e).into()))?;

        // Run migrations once against a freshly checked-out connection.
        {
            let conn = pool
                .get()
                .map_err(|e| rusqlite::Error::InvalidPath(format!("pool get: {}", e).into()))?;
            schema::run_migrations(&conn)?;
        }

        Ok(Self { pool })
    }

    /// Grab a connection from the pool. Panics if the pool is exhausted —
    /// which shouldn't happen with max_size=8 and scoped borrows, but if it
    /// ever does it's a real bug worth failing loudly.
    pub fn conn(&self) -> DbConn {
        self.pool
            .get()
            .expect("database pool exhausted or poisoned")
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
