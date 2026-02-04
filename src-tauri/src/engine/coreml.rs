//! CoreML backend for Parakeet TDT inference via sidecar
//!
//! This backend uses the parakeet-coreml sidecar (Swift/FluidAudio) for inference.
//! Much simpler and more performant than direct FFI approach.

use crate::engine::config::DecodingConfig;
use crate::engine::{ASREngine, TranscriptionLanguage};
use crate::error::{AppError, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::{debug, info, warn};

/// Result from the sidecar
#[derive(Debug, Deserialize)]
struct SidecarResult {
    text: String,
    confidence: f64,
    processing_time_ms: i64,
}

/// Error from the sidecar
#[derive(Debug, Deserialize)]
struct SidecarError {
    error: String,
}

/// CoreML engine using sidecar for inference
pub struct CoreMLEngine {
    model_dir: Option<PathBuf>,
    sidecar_path: Option<PathBuf>,
}

unsafe impl Send for CoreMLEngine {}
unsafe impl Sync for CoreMLEngine {}

impl CoreMLEngine {
    pub fn new() -> Self {
        Self {
            model_dir: None,
            sidecar_path: None,
        }
    }

    /// Find the sidecar binary
    fn find_sidecar() -> Option<PathBuf> {
        // In development: next to the executable or in src-tauri/binaries
        let exe_dir = std::env::current_exe().ok()?.parent()?.to_path_buf();

        // Check various locations
        let candidates = [
            // Tauri bundle location
            exe_dir.join("parakeet-coreml"),
            // Development location (relative to src-tauri)
            PathBuf::from("binaries/parakeet-coreml-x86_64-apple-darwin"),
            PathBuf::from("../src-tauri/binaries/parakeet-coreml-x86_64-apple-darwin"),
            // Absolute development path
            PathBuf::from("/Users/herve/dev/mvpparakeet/wakascribe/src-tauri/binaries/parakeet-coreml-x86_64-apple-darwin"),
        ];

        for path in &candidates {
            if path.exists() {
                info!("Found sidecar at: {:?}", path);
                return Some(path.clone());
            }
        }

        // Try using tauri's sidecar resolution
        #[cfg(feature = "tauri")]
        if let Ok(sidecar) = tauri::api::process::Command::new_sidecar("parakeet-coreml") {
            // This would work in a Tauri context
        }

        None
    }

    /// Write audio samples to a temporary WAV file
    fn write_temp_wav(&self, samples: &[f32]) -> Result<PathBuf> {
        let temp_path = std::env::temp_dir().join(format!("wakascribe_audio_{}.wav", std::process::id()));

        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 16000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let mut writer = hound::WavWriter::create(&temp_path, spec)
            .map_err(|e| AppError::Transcription(format!("Failed to create temp WAV: {}", e)))?;

        for &sample in samples {
            let sample_i16 = (sample * 32767.0).clamp(-32768.0, 32767.0) as i16;
            writer.write_sample(sample_i16)
                .map_err(|e| AppError::Transcription(format!("Failed to write sample: {}", e)))?;
        }

        writer.finalize()
            .map_err(|e| AppError::Transcription(format!("Failed to finalize WAV: {}", e)))?;

        debug!("Wrote temp WAV: {:?} ({} samples)", temp_path, samples.len());
        Ok(temp_path)
    }

    /// Call the sidecar and parse the result
    fn call_sidecar(&self, audio_path: &Path) -> Result<SidecarResult> {
        let sidecar_path = self.sidecar_path.as_ref()
            .ok_or_else(|| AppError::Transcription("Sidecar not found".to_string()))?;

        let model_dir = self.model_dir.as_ref()
            .ok_or_else(|| AppError::Transcription("Model directory not set".to_string()))?;

        debug!("Calling sidecar: {:?} {:?} --models {:?}", sidecar_path, audio_path, model_dir);

        let output = Command::new(sidecar_path)
            .arg(audio_path)
            .arg("--models")
            .arg(model_dir)
            .output()
            .map_err(|e| AppError::Transcription(format!("Failed to run sidecar: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if !stderr.is_empty() {
            debug!("Sidecar stderr: {}", stderr);
        }

        if !output.status.success() {
            // Try to parse error JSON
            if let Ok(error) = serde_json::from_str::<SidecarError>(&stdout) {
                return Err(AppError::Transcription(error.error));
            }
            return Err(AppError::Transcription(format!(
                "Sidecar failed with exit code {:?}: {}",
                output.status.code(),
                stdout
            )));
        }

        // Parse the last line as JSON (skip FluidAudio logs)
        let json_line = stdout.lines().last().unwrap_or(&stdout);

        serde_json::from_str::<SidecarResult>(json_line)
            .map_err(|e| AppError::Transcription(format!("Failed to parse sidecar output: {} - raw: {}", e, json_line)))
    }
}

impl Default for CoreMLEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl ASREngine for CoreMLEngine {
    fn name(&self) -> &str {
        "CoreML"
    }

    fn is_loaded(&self) -> bool {
        self.model_dir.is_some() && self.sidecar_path.is_some()
    }

    fn load_model(&mut self, model_dir: &Path) -> Result<()> {
        info!("Loading CoreML sidecar engine, model_dir: {:?}", model_dir);

        // Find sidecar binary
        let sidecar_path = Self::find_sidecar()
            .ok_or_else(|| AppError::Transcription("CoreML sidecar binary not found".to_string()))?;

        // Verify model directory exists
        if !model_dir.exists() {
            return Err(AppError::Transcription(format!(
                "Model directory not found: {:?}",
                model_dir
            )));
        }

        self.model_dir = Some(model_dir.to_path_buf());
        self.sidecar_path = Some(sidecar_path);

        info!("CoreML sidecar engine ready");
        Ok(())
    }

    fn run_inference(
        &self,
        samples: &[f32],
        _language: TranscriptionLanguage,
        _config: &DecodingConfig,
    ) -> Result<String> {
        info!(
            "Starting CoreML sidecar inference on {} samples ({:.2}s)",
            samples.len(),
            samples.len() as f32 / 16000.0
        );

        // Write audio to temp file
        let temp_wav = self.write_temp_wav(samples)?;

        // Call sidecar
        let result = self.call_sidecar(&temp_wav);

        // Clean up temp file
        if let Err(e) = std::fs::remove_file(&temp_wav) {
            warn!("Failed to remove temp WAV: {}", e);
        }

        let result = result?;

        info!(
            "CoreML transcription: confidence={:.2}%, time={}ms",
            result.confidence * 100.0,
            result.processing_time_ms
        );

        Ok(result.text)
    }
}
