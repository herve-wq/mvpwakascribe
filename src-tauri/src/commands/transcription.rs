use crate::audio::{duration_ms, load_audio_file, resample_to_16k};
use crate::commands::audio::AudioState;
use crate::engine::ParakeetEngine;
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
) -> Result<Transcription> {
    let samples = audio_state.0.stop()?;
    let sample_rate = audio_state.0.sample_rate();

    // Resample to 16kHz
    let resampled = resample_to_16k(&samples, sample_rate)?;

    // Transcribe
    let mut engine = engine_state.0.lock();
    let transcription = engine.transcribe(&resampled, "dictation", None)?;

    // Save to database
    storage::with_db(|conn| insert_transcription(conn, &transcription))?;

    Ok(transcription)
}

#[tauri::command]
pub async fn transcribe_file(
    window: Window,
    engine_state: State<'_, EngineState>,
    file_path: String,
) -> Result<Transcription> {
    let path = PathBuf::from(&file_path);

    if !path.exists() {
        return Err(AppError::NotFound(format!("File not found: {}", file_path)));
    }

    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .map(String::from);

    info!("Transcribing file: {:?}", path);

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

    // Transcribe
    let mut engine = engine_state.0.lock();
    let transcription = engine.transcribe(&resampled, "file", file_name)?;

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
