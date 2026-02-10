use crate::audio::{duration_ms, load_audio_file, prepare_audio};
use crate::commands::audio::AudioState;
use crate::engine::{DecodingConfig, DynamicEngine, EngineBackend, TranscriptionLanguage};
use crate::error::{AppError, Result};
use crate::storage::{
    self, insert_transcription, Transcription, TranscriptionProgress,
};
use parking_lot::Mutex;
use serde::Serialize;
use std::path::PathBuf;
use tauri::{Emitter, State, Window};
use tracing::info;

/// State wrapper for the ASR engine (supports dynamic backend switching)
pub struct EngineState(pub Mutex<DynamicEngine>);

/// State for the model base path (needed for backend switching)
pub struct ModelPathState(pub PathBuf);

/// Shared state for engine loading error messages
pub struct EngineErrorState(pub Mutex<Option<String>>);

/// Engine status returned to the frontend
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineStatus {
    backend: String,
    is_loaded: bool,
    error: Option<String>,
}

/// Get the current engine status (backend, loaded, error)
#[tauri::command]
pub fn get_engine_status(
    engine_state: State<'_, EngineState>,
    error_state: State<'_, EngineErrorState>,
) -> EngineStatus {
    let engine = engine_state.0.lock();
    let error = error_state.0.lock();
    EngineStatus {
        backend: engine.backend().display_name().to_string(),
        is_loaded: engine.is_loaded(),
        error: error.clone(),
    }
}

#[tauri::command]
pub fn stop_recording(
    audio_state: State<'_, AudioState>,
    engine_state: State<'_, EngineState>,
    language: Option<TranscriptionLanguage>,
    decoding_config: Option<DecodingConfig>,
) -> Result<Transcription> {
    // Guard: engine must be loaded
    {
        let engine = engine_state.0.lock();
        if !engine.is_loaded() {
            return Err(AppError::InvalidState("Moteur non charge".into()));
        }
    }

    let samples = audio_state.0.stop()?;
    let sample_rate = audio_state.0.sample_rate();

    // Resample to 16kHz and normalize
    let normalized = prepare_audio(&samples, sample_rate)?;

    // Use provided language or default to Auto
    let lang = language.unwrap_or_default();

    // Transcribe
    let engine = engine_state.0.lock();
    let transcription = engine.transcribe(&normalized, "dictation", None, lang, decoding_config)?;

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
    decoding_config: Option<DecodingConfig>,
) -> Result<Transcription> {
    // Guard: engine must be loaded
    {
        let engine = engine_state.0.lock();
        if !engine.is_loaded() {
            return Err(AppError::InvalidState("Moteur non charge".into()));
        }
    }

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
    let config = decoding_config.clone();
    info!(
        "Transcribing file: {:?} with language: {:?}, decoding_config: {:?}",
        path, lang, config
    );

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

    // Resample to 16kHz and normalize
    let normalized = prepare_audio(&samples, sample_rate)?;

    // Transcribe
    let engine = engine_state.0.lock();
    let transcription = engine.transcribe(&normalized, "file", file_name, lang, decoding_config)?;

    // Final progress with real speed factor
    let speed_factor = transcription.duration_ms as f64 / transcription.processing_time_ms.max(1) as f64;
    let _ = window.emit(
        "transcription-progress",
        TranscriptionProgress {
            current_ms: total_ms,
            total_ms,
            speed_factor,
        },
    );

    // Save to database
    storage::with_db(|conn| insert_transcription(conn, &transcription))?;

    Ok(transcription)
}

#[tauri::command]
pub fn get_transcription(id: String) -> Result<Transcription> {
    storage::with_db(|conn| storage::get_transcription_or_error(conn, &id))
}

/// Switch to a different inference backend
#[tauri::command]
pub fn switch_engine_backend(
    engine_state: State<'_, EngineState>,
    model_path_state: State<'_, ModelPathState>,
    error_state: State<'_, EngineErrorState>,
    backend: String,
) -> Result<String> {
    let backend = match backend.as_str() {
        "openvino" => EngineBackend::OpenVINO,
        #[cfg(target_os = "macos")]
        "coreml" => EngineBackend::CoreML,
        _ => return Err(AppError::InvalidInput(format!("Unknown backend: {}", backend))),
    };

    let model_dir = model_path_state.0.join(backend.model_subdir());
    if !model_dir.exists() {
        let err_msg = format!(
            "Model directory not found for {}: {:?}",
            backend.display_name(),
            model_dir
        );
        *error_state.0.lock() = Some(err_msg.clone());
        return Err(AppError::NotFound(err_msg));
    }

    let mut engine = engine_state.0.lock();
    match engine.switch_backend(backend, &model_dir) {
        Ok(_) => {
            *error_state.0.lock() = None;
            info!("Switched to {} backend", backend.display_name());
            Ok(backend.display_name().to_string())
        }
        Err(e) => {
            let err_msg = format!("Failed to load {}: {}", backend.display_name(), e);
            *error_state.0.lock() = Some(err_msg);
            Err(e)
        }
    }
}

/// Get the current engine backend name
#[tauri::command]
pub fn get_engine_backend(engine_state: State<'_, EngineState>) -> String {
    let engine = engine_state.0.lock();
    engine.backend().display_name().to_string()
}
