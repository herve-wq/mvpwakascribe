use crate::audio::{duration_ms, load_audio_file, normalize_audio, resample_to_16k};
use crate::commands::audio::AudioState;
use crate::engine::{ParakeetEngine, TranscriptionLanguage};
use crate::error::{AppError, Result};
use crate::storage::{
    self, insert_transcription, Transcription, TranscriptionProgress,
};
use parking_lot::Mutex;
use std::path::PathBuf;
use tauri::{Emitter, State, Window};
use tracing::info;

pub struct EngineState(pub Mutex<ParakeetEngine>);

#[tauri::command]
pub fn stop_recording(
    audio_state: State<'_, AudioState>,
    engine_state: State<'_, EngineState>,
    language: Option<TranscriptionLanguage>,
) -> Result<Transcription> {
    let samples = audio_state.0.stop()?;
    let sample_rate = audio_state.0.sample_rate();

    // Resample to 16kHz
    let resampled = resample_to_16k(&samples, sample_rate)?;

    // Normalize audio level for consistent transcription
    let (normalized, _gain) = normalize_audio(&resampled);

    // Use provided language or default to Auto
    let lang = language.unwrap_or_default();

    // Transcribe
    let engine = engine_state.0.lock();
    let transcription = engine.transcribe(&normalized, "dictation", None, lang)?;

    // Save to database
    storage::with_db(|conn| insert_transcription(conn, &transcription))?;

    Ok(transcription)
}

#[tauri::command]
pub async fn transcribe_file(
    window: Window,
    engine_state: State<'_, EngineState>,
    file_path: String,
    language: Option<TranscriptionLanguage>,
) -> Result<Transcription> {
    let path = PathBuf::from(&file_path);

    if !path.exists() {
        return Err(AppError::NotFound(format!("File not found: {}", file_path)));
    }

    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .map(String::from);

    // Use provided language or default to Auto
    let lang = language.unwrap_or_default();
    info!("Transcribing file: {:?} with language: {:?}", path, lang);

    // Load and process audio
    let (samples, sample_rate) = load_audio_file(&path)?;
    let total_ms = duration_ms(&samples, sample_rate);

    // Emit initial progress
    let _ = window.emit(
        "transcription-progress",
        TranscriptionProgress {
            current_ms: 0,
            total_ms,
            speed_factor: 0.0,
        },
    );

    // Resample to 16kHz
    let resampled = resample_to_16k(&samples, sample_rate)?;

    // Normalize audio level for consistent transcription
    let (normalized, _gain) = normalize_audio(&resampled);

    // Transcribe
    let engine = engine_state.0.lock();
    let transcription = engine.transcribe(&normalized, "file", file_name, lang)?;

    // Final progress
    let _ = window.emit(
        "transcription-progress",
        TranscriptionProgress {
            current_ms: total_ms,
            total_ms,
            speed_factor: 4.0, // Mock speed
        },
    );

    // Save to database
    storage::with_db(|conn| insert_transcription(conn, &transcription))?;

    Ok(transcription)
}

#[tauri::command]
pub fn get_transcription(id: String) -> Result<Transcription> {
    storage::with_db(|conn| {
        storage::get_transcription(conn, &id)?
            .ok_or_else(|| AppError::NotFound(format!("Transcription not found: {}", id)))
    })
}
