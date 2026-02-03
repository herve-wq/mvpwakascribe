use crate::audio::{resample_to_16k, write_wav, AudioCapture};
use crate::error::Result;
use crate::storage::AudioDevice;
use std::path::PathBuf;
use tauri::{AppHandle, Manager, State};
use tracing::info;

pub struct AudioState(pub AudioCapture);

#[tauri::command]
pub fn list_audio_devices() -> Result<Vec<AudioDevice>> {
    AudioCapture::list_devices()
}

#[tauri::command]
pub fn start_recording(
    state: State<'_, AudioState>,
    device_id: Option<String>,
) -> Result<()> {
    state.0.start(device_id.as_deref())
}

#[tauri::command]
pub fn pause_recording(state: State<'_, AudioState>) -> Result<()> {
    state.0.pause()
}

#[tauri::command]
pub fn resume_recording(state: State<'_, AudioState>) -> Result<()> {
    state.0.resume()
}

#[tauri::command]
pub fn get_audio_level(state: State<'_, AudioState>) -> f32 {
    state.0.get_audio_level()
}

#[tauri::command]
pub fn stop_recording_to_wav(
    app: AppHandle,
    state: State<'_, AudioState>,
) -> Result<String> {
    // Stop recording and get samples
    let samples = state.0.stop()?;
    let sample_rate = state.0.sample_rate();

    info!(
        "Recording stopped: {} samples at {}Hz",
        samples.len(),
        sample_rate
    );

    // Resample to 16kHz if needed
    let resampled = resample_to_16k(&samples, sample_rate)?;

    // Determine output path: model/test_audio.wav
    let resource_path = app
        .path()
        .resource_dir()
        .unwrap_or_else(|_| PathBuf::from("."));

    // Try model directory relative to resource, or fall back to cwd
    let model_dir = if resource_path.join("model").exists() {
        resource_path.join("model")
    } else {
        // Development mode: use project root model directory
        let mut path = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        // If we're in src-tauri, go up one level
        if path.ends_with("src-tauri") {
            path.pop();
        }
        path.join("model")
    };

    let output_path = model_dir.join("test_audio.wav");

    info!("Writing WAV to: {}", output_path.display());

    // Write WAV file
    write_wav(&resampled, &output_path)?;

    Ok(output_path.to_string_lossy().to_string())
}
