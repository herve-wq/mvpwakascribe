use crate::error::Result;
use crate::storage::{self, Transcription};

#[tauri::command]
pub fn list_transcriptions() -> Result<Vec<Transcription>> {
    storage::with_db(|conn| storage::list_transcriptions(conn))
}

#[tauri::command]
pub fn delete_transcription(id: String) -> Result<()> {
    storage::with_db(|conn| storage::delete_transcription(conn, &id))
}

#[tauri::command]
pub fn update_transcription_text(id: String, edited_text: String) -> Result<()> {
    storage::with_db(|conn| storage::update_transcription_text(conn, &id, &edited_text))
}
