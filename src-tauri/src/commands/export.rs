use crate::error::{AppError, Result};
use crate::export;
use crate::storage;
use std::path::PathBuf;
use tauri_plugin_clipboard_manager::ClipboardExt;

#[tauri::command]
pub fn export_to_txt(id: String, path: String) -> Result<()> {
    let transcription = storage::with_db(|conn| {
        storage::get_transcription(conn, &id)?
            .ok_or_else(|| AppError::NotFound(format!("Transcription not found: {}", id)))
    })?;

    export::export_to_txt(&transcription, &PathBuf::from(path))
}

#[tauri::command]
pub fn export_to_docx(id: String, path: String) -> Result<()> {
    let transcription = storage::with_db(|conn| {
        storage::get_transcription(conn, &id)?
            .ok_or_else(|| AppError::NotFound(format!("Transcription not found: {}", id)))
    })?;

    export::export_to_docx(&transcription, &PathBuf::from(path))
}

#[tauri::command]
pub fn copy_to_clipboard(app: tauri::AppHandle, text: String) -> Result<()> {
    app.clipboard()
        .write_text(text)
        .map_err(|e| AppError::Export(e.to_string()))
}
