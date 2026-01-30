use crate::error::Result;
use crate::storage::{self, Settings};

#[tauri::command]
pub fn get_settings() -> Result<Settings> {
    storage::with_db(|conn| storage::get_settings(conn))
}

#[tauri::command]
pub fn update_settings(settings: Settings) -> Result<()> {
    storage::with_db(|conn| storage::update_settings(conn, &settings))
}
