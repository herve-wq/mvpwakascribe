use crate::audio::vad::{find_best_cut_point, VadConfig};
use crate::audio::{calculate_rms, duration_ms, load_audio_file, normalize_audio, prepare_audio, resample_to_16k, trim_trailing_silence};
use crate::commands::audio::AudioState;
use crate::engine::{
    filter_chunk_hallucinations, DecodingConfig, DynamicEngine, EngineBackend,
    TranscriptionLanguage,
};
use crate::error::{AppError, Result};
use crate::storage::{
    self, insert_transcription, Segment, StreamingSegment, Transcription, TranscriptionProgress,
};
use parking_lot::Mutex;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{Emitter, State, Window};
use tracing::{info, warn};
use uuid::Uuid;

/// State wrapper for the ASR engine (supports dynamic backend switching)
pub struct EngineState(pub Mutex<DynamicEngine>);

/// State for the model base path (needed for backend switching)
pub struct ModelPathState(pub PathBuf);

/// Shared state for engine loading error messages
pub struct EngineErrorState(pub Mutex<Option<String>>);

/// Controls the streaming transcription loop (true = running)
pub struct StreamingState(pub Arc<AtomicBool>);

/// Notified when the streaming loop has fully exited
pub struct StreamingDone(pub Arc<tokio::sync::Notify>);

/// Accumulates segments produced during streaming transcription
pub struct StreamingSegments(pub Mutex<Vec<Segment>>);

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
pub async fn stop_recording(
    window: Window,
    audio_state: State<'_, AudioState>,
    engine_state: State<'_, EngineState>,
    streaming_state: State<'_, StreamingState>,
    streaming_done: State<'_, StreamingDone>,
    streaming_segments: State<'_, StreamingSegments>,
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

    // 1. Signal the streaming loop to stop
    streaming_state.0.store(false, Ordering::SeqCst);

    // 2. Stop audio capture and get remaining buffer
    let samples = audio_state.0.stop()?;
    let sample_rate = audio_state.0.sample_rate();

    // 3. Wait for the streaming loop to fully exit (max 15s safety timeout)
    //    This ensures any in-flight inference finishes and pushes its segment
    let notify = streaming_done.0.clone();
    let _ = tokio::time::timeout(
        std::time::Duration::from_secs(15),
        notify.notified(),
    )
    .await;
    info!("Streaming loop confirmed stopped");

    let lang = language.unwrap_or_default();
    let config = decoding_config.unwrap_or_default();

    // 4. Take all segments accumulated during streaming (loop is done, no race)
    let mut accumulated_segments = {
        let mut segs = streaming_segments.0.lock();
        std::mem::take(&mut *segs)
    };

    // 5. Transcribe remaining audio (the queue) if > 1s
    let mut normalized = prepare_audio(&samples, sample_rate)?;
    // Trim trailing silence from the remaining queue too
    let keep = trim_trailing_silence(&normalized, SILENCE_RATIO);
    if keep < normalized.len() {
        info!(
            "Trimmed trailing silence from queue: {:.2}s -> {:.2}s",
            normalized.len() as f64 / 16000.0,
            keep as f64 / 16000.0
        );
        normalized.truncate(keep);
    }
    if normalized.len() > 16000 {
        info!(
            "Transcribing remaining queue: {} samples ({:.1}s)",
            normalized.len(),
            normalized.len() as f64 / 16000.0
        );
        let mut engine = engine_state.0.lock();
        let text = engine.transcribe_chunk(&normalized, lang, &config)?;
        let text = filter_chunk_hallucinations(&text);
        if !text.is_empty() {
            let duration_ms = (normalized.len() as f64 / 16000.0 * 1000.0) as i64;
            let offset_ms = accumulated_segments
                .last()
                .map(|s| s.end_ms)
                .unwrap_or(0);
            let segment = Segment {
                id: Uuid::new_v4().to_string(),
                start_ms: offset_ms,
                end_ms: offset_ms + duration_ms,
                text: text.clone(),
                confidence: 0.9,
            };
            let _ = window.emit(
                "transcription-segment",
                StreamingSegment {
                    text: text.clone(),
                    is_final: true,
                    confidence: Some(0.9),
                },
            );
            accumulated_segments.push(segment);
        }
    } else if !normalized.is_empty() {
        info!(
            "Remaining queue too short ({} samples, {:.1}s), skipping",
            normalized.len(),
            normalized.len() as f64 / 16000.0
        );
    }

    // 6. Assemble final Transcription from all segments
    let raw_text = accumulated_segments
        .iter()
        .map(|s| s.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    let total_duration_ms = accumulated_segments
        .last()
        .map(|s| s.end_ms)
        .unwrap_or(0);
    let now = chrono::Utc::now().to_rfc3339();

    let transcription = Transcription {
        id: Uuid::new_v4().to_string(),
        created_at: now.clone(),
        updated_at: now,
        source_type: "dictation".to_string(),
        source_name: None,
        duration_ms: total_duration_ms,
        language: lang.code().to_string(),
        segments: accumulated_segments,
        raw_text,
        edited_text: None,
        is_edited: false,
        processing_time_ms: 0,
    };

    // Save to database
    storage::with_db(|conn| insert_transcription(conn, &transcription))?;

    Ok(transcription)
}

/// Minimum samples to bother transcribing a chunk (3 seconds at 16kHz)
const MIN_CHUNK_SAMPLES: usize = 48000;

/// Relative silence ratio: a leftover or trailing window is considered silence
/// if its RMS is below this fraction of the chunk's overall RMS.
/// Adapts automatically to recording level (quiet mic → low RMS, loud mic → high RMS).
const SILENCE_RATIO: f32 = 0.1;

#[tauri::command]
pub async fn start_streaming_transcription(
    window: Window,
    audio_state: State<'_, AudioState>,
    engine_state: State<'_, EngineState>,
    streaming_state: State<'_, StreamingState>,
    streaming_done: State<'_, StreamingDone>,
    streaming_segments: State<'_, StreamingSegments>,
    language: Option<TranscriptionLanguage>,
    decoding_config: Option<DecodingConfig>,
) -> Result<()> {
    // Reset streaming state
    streaming_state.0.store(true, Ordering::SeqCst);
    streaming_segments.0.lock().clear();
    let done_notify = streaming_done.0.clone();

    let lang = language.unwrap_or_default();
    let config = decoding_config.unwrap_or_default();
    // Leftover kept in 16kHz RAW domain (resampled but NOT normalized)
    // Normalization is applied on the full chunk just before inference
    let mut leftover_16k: Vec<f32> = Vec::new();
    let mut segment_offset_ms: i64 = 0;
    let vad_config = VadConfig::default();

    info!("Streaming transcription started (language: {:?})", lang);

    // Loop until stopped
    while streaming_state.0.load(Ordering::SeqCst) {
        // Sleep 10 seconds between drain cycles
        tokio::time::sleep(std::time::Duration::from_secs(10)).await;

        // Check again after sleep (may have been stopped during sleep)
        if !streaming_state.0.load(Ordering::SeqCst) {
            break;
        }

        // Drain accumulated audio from capture buffer
        let sample_rate = audio_state.0.sample_rate();
        let max_drain = sample_rate as usize * 15; // max 15s at source rate
        let drained = match audio_state.0.drain_chunk(max_drain) {
            Ok(d) => d,
            Err(e) => {
                warn!("drain_chunk failed: {}", e);
                continue;
            }
        };

        if drained.is_empty() && leftover_16k.is_empty() {
            continue;
        }

        // Resample new audio to 16kHz (without normalizing), then prepend leftover
        let mut raw_16k = std::mem::take(&mut leftover_16k);
        if !drained.is_empty() {
            match resample_to_16k(&drained, sample_rate) {
                Ok(r) => raw_16k.extend_from_slice(&r),
                Err(e) => {
                    warn!("resample failed: {}", e);
                    leftover_16k = raw_16k;
                    continue;
                }
            }
        }

        // If < 3s at 16kHz, save as leftover for next cycle
        if raw_16k.len() < MIN_CHUNK_SAMPLES {
            leftover_16k = raw_16k;
            continue;
        }

        // Find a silence point to cut cleanly (avoid mid-word splits)
        // Search in the last 3 seconds for the quietest spot
        let search_margin = 16000 * 3; // 3s at 16kHz
        let mut to_normalize;
        if raw_16k.len() > MIN_CHUNK_SAMPLES + search_margin {
            let search_start = raw_16k.len() - search_margin;
            let (cut, _rms, is_silence) =
                find_best_cut_point(&raw_16k, search_start, raw_16k.len(), &vad_config);
            to_normalize = raw_16k[..cut].to_vec();
            let candidate_leftover = &raw_16k[cut..];
            let chunk_rms = calculate_rms(&to_normalize);
            let leftover_rms = calculate_rms(candidate_leftover);
            let threshold = chunk_rms * SILENCE_RATIO;
            info!(
                "VAD cut at {:.2}s (silence={}), leftover {:.2}s RMS={:.4} (chunk RMS={:.4}, threshold={:.4})",
                cut as f64 / 16000.0,
                is_silence,
                candidate_leftover.len() as f64 / 16000.0,
                leftover_rms,
                chunk_rms,
                threshold
            );
            if leftover_rms >= threshold {
                leftover_16k = candidate_leftover.to_vec();
            } else {
                info!("Discarding silent leftover (RMS {:.4} < {:.4})", leftover_rms, threshold);
                leftover_16k = Vec::new();
            }
        } else {
            to_normalize = raw_16k;
        }

        // Trim trailing silence to prevent decoder hallucinations on silent frames
        let keep = trim_trailing_silence(&to_normalize, SILENCE_RATIO);
        if keep < to_normalize.len() {
            info!(
                "Trimmed trailing silence: {:.2}s -> {:.2}s",
                to_normalize.len() as f64 / 16000.0,
                keep as f64 / 16000.0
            );
            to_normalize.truncate(keep);
        }

        // Normalize the full chunk right before inference (consistent gain)
        let (to_infer, _gain) = normalize_audio(&to_normalize);

        // Run inference (this acquires the engine mutex)
        let text = {
            let mut engine = engine_state.0.lock();
            match engine.transcribe_chunk(&to_infer, lang, &config) {
                Ok(t) => t,
                Err(e) => {
                    warn!("Chunk inference failed: {}", e);
                    // Prepend failed chunk back to leftover (raw, not normalized)
                    let mut combined = to_normalize;
                    combined.append(&mut leftover_16k);
                    leftover_16k = combined;
                    continue;
                }
            }
        };

        let text = filter_chunk_hallucinations(&text);
        if text.is_empty() {
            continue;
        }

        let duration_ms = (to_infer.len() as f64 / 16000.0 * 1000.0) as i64;
        let segment = Segment {
            id: Uuid::new_v4().to_string(),
            start_ms: segment_offset_ms,
            end_ms: segment_offset_ms + duration_ms,
            text: text.clone(),
            confidence: 0.9,
        };
        segment_offset_ms += duration_ms;

        // Emit to frontend
        let _ = window.emit(
            "transcription-segment",
            StreamingSegment {
                text: text.clone(),
                is_final: true,
                confidence: Some(0.9),
            },
        );

        // Accumulate segment
        streaming_segments.0.lock().push(segment);

        info!("Streamed segment: '{}' (offset {}ms)", text, segment_offset_ms);
    }

    info!("Streaming transcription loop ended");
    done_notify.notify_one();
    Ok(())
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
    let mut engine = engine_state.0.lock();
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
