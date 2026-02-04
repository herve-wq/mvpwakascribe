use crate::audio::{duration_ms, load_audio_file, normalize_audio, resample_to_16k};
use crate::commands::audio::AudioState;
use crate::engine::{DecodingConfig, DynamicEngine, EngineBackend, TranscriptionLanguage};
use crate::error::{AppError, Result};
use crate::storage::{
    self, insert_transcription, Transcription, TranscriptionProgress,
};
use parking_lot::Mutex;
use std::path::PathBuf;
use tauri::{Emitter, State, Window};
use tracing::info;

/// State wrapper for the ASR engine (supports dynamic backend switching)
pub struct EngineState(pub Mutex<DynamicEngine>);

/// State for the model base path (needed for backend switching)
pub struct ModelPathState(pub PathBuf);

#[tauri::command]
pub fn stop_recording(
    audio_state: State<'_, AudioState>,
    engine_state: State<'_, EngineState>,
    language: Option<TranscriptionLanguage>,
    decoding_config: Option<DecodingConfig>,
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

    // Resample to 16kHz
    let resampled = resample_to_16k(&samples, sample_rate)?;

    // Normalize audio level for consistent transcription
    let (normalized, _gain) = normalize_audio(&resampled);

    // Transcribe
    let engine = engine_state.0.lock();
    let transcription = engine.transcribe(&normalized, "file", file_name, lang, decoding_config)?;

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

/// Switch to a different inference backend
#[tauri::command]
pub fn switch_engine_backend(
    engine_state: State<'_, EngineState>,
    model_path_state: State<'_, ModelPathState>,
    backend: String,
) -> Result<String> {
    let backend = match backend.as_str() {
        "openvino" => EngineBackend::OpenVINO,
        "onnxruntime" => EngineBackend::OnnxRuntime,
        #[cfg(target_os = "macos")]
        "coreml" => EngineBackend::CoreML,
        _ => return Err(AppError::InvalidInput(format!("Unknown backend: {}", backend))),
    };

    let model_dir = model_path_state.0.join(backend.model_subdir());
    if !model_dir.exists() {
        return Err(AppError::NotFound(format!(
            "Model directory not found for {}: {:?}",
            backend.display_name(),
            model_dir
        )));
    }

    let mut engine = engine_state.0.lock();
    engine.switch_backend(backend, &model_dir)?;

    info!("Switched to {} backend", backend.display_name());
    Ok(backend.display_name().to_string())
}

/// Get the current engine backend name
#[tauri::command]
pub fn get_engine_backend(engine_state: State<'_, EngineState>) -> String {
    let engine = engine_state.0.lock();
    engine.backend().display_name().to_string()
}
