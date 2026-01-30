use crate::error::{AppError, Result};
use once_cell::sync::OnceCell;
use parking_lot::Mutex;
use rusqlite::Connection;
use std::path::PathBuf;
use tracing::info;

static DB: OnceCell<Mutex<Connection>> = OnceCell::new();

fn get_db_path() -> PathBuf {
    let app_dir = dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("com.wakascribe.app");

    std::fs::create_dir_all(&app_dir).ok();
    app_dir.join("wakascribe.db")
}

pub fn init_database() -> Result<()> {
    let db_path = get_db_path();
    info!("Initializing database at {:?}", db_path);

    let conn = Connection::open(&db_path)?;

    // Run migrations
    conn.execute_batch(include_str!("../../migrations/001_init.sql"))?;

    DB.set(Mutex::new(conn))
        .map_err(|_| AppError::InvalidState("Database already initialized".into()))?;

    Ok(())
}

pub fn with_db<F, T>(f: F) -> Result<T>
where
    F: FnOnce(&Connection) -> Result<T>,
{
    let db = DB
        .get()
        .ok_or_else(|| AppError::InvalidState("Database not initialized".into()))?;
    let conn = db.lock();
    f(&conn)
}

pub fn with_db_mut<F, T>(f: F) -> Result<T>
where
    F: FnOnce(&mut Connection) -> Result<T>,
{
    let db = DB
        .get()
        .ok_or_else(|| AppError::InvalidState("Database not initialized".into()))?;
    let mut conn = db.lock();
    f(&mut conn)
}

// Add dirs dependency for cross-platform paths
mod dirs {
    use std::path::PathBuf;

    pub fn data_local_dir() -> Option<PathBuf> {
        #[cfg(target_os = "macos")]
        {
            std::env::var_os("HOME")
                .map(PathBuf::from)
                .map(|p| p.join("Library/Application Support"))
        }

        #[cfg(target_os = "windows")]
        {
            std::env::var_os("LOCALAPPDATA").map(PathBuf::from)
        }

        #[cfg(target_os = "linux")]
        {
            std::env::var_os("XDG_DATA_HOME")
                .map(PathBuf::from)
                .or_else(|| {
                    std::env::var_os("HOME")
                        .map(PathBuf::from)
                        .map(|p| p.join(".local/share"))
                })
        }
    }
}
