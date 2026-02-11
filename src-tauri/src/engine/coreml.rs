//! CoreML backend for Parakeet TDT inference via persistent sidecar
//!
//! The sidecar is a long-running Swift process that loads CoreML models once
//! and accepts transcription requests via stdin/stdout JSON-lines protocol.

use crate::engine::config::DecodingConfig;
use crate::engine::{ASREngine, TranscriptionLanguage};
use crate::error::{AppError, Result};
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::time::Duration;
use tracing::{debug, info, warn};

/// Request sent to the sidecar via stdin
#[derive(Debug, Serialize)]
struct SidecarRequest {
    audio_path: String,
    language: String,
    beam_width: usize,
    temperature: f32,
    blank_penalty: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    command: Option<String>,
}

/// Result from the sidecar
#[derive(Debug, Deserialize)]
struct SidecarResult {
    text: Option<String>,
    confidence: Option<f64>,
    processing_time_ms: Option<i64>,
    // Ready message
    status: Option<String>,
    // Error message
    error: Option<String>,
}

/// CoreML engine using a persistent sidecar for inference
pub struct CoreMLEngine {
    model_dir: Option<PathBuf>,
    sidecar_path: Option<PathBuf>,
    child: Option<Child>,
    stdin: Option<ChildStdin>,
    stdout_reader: Option<BufReader<ChildStdout>>,
}

unsafe impl Send for CoreMLEngine {}
unsafe impl Sync for CoreMLEngine {}

impl CoreMLEngine {
    pub fn new() -> Self {
        Self {
            model_dir: None,
            sidecar_path: None,
            child: None,
            stdin: None,
            stdout_reader: None,
        }
    }

    /// Find the sidecar binary
    fn find_sidecar() -> Option<PathBuf> {
        let exe_dir = std::env::current_exe().ok()?.parent()?.to_path_buf();

        // Bundled by Tauri via externalBin (production + dev)
        let bundled = exe_dir.join("parakeet-coreml");
        if bundled.exists() {
            info!("Found sidecar at: {:?}", bundled);
            return Some(bundled);
        }

        // Fallback: src-tauri/binaries/ for manual testing
        let dev_path = PathBuf::from("binaries/parakeet-coreml");
        if dev_path.exists() {
            info!("Found sidecar at: {:?}", dev_path);
            return Some(dev_path);
        }

        None
    }

    /// Spawn the sidecar process and wait for the "ready" message
    fn spawn_sidecar(&mut self) -> Result<()> {
        let sidecar_path = self.sidecar_path.as_ref()
            .ok_or_else(|| AppError::Transcription("Sidecar path not set".to_string()))?
            .clone();

        info!("Spawning persistent sidecar: {:?}", sidecar_path);

        let mut child = Command::new(&sidecar_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| AppError::Transcription(format!("Failed to spawn sidecar: {}", e)))?;

        let stdin = child.stdin.take()
            .ok_or_else(|| AppError::Transcription("Failed to capture sidecar stdin".to_string()))?;
        let stdout = child.stdout.take()
            .ok_or_else(|| AppError::Transcription("Failed to capture sidecar stdout".to_string()))?;

        // Spawn a thread to drain stderr (logs) so the pipe doesn't block
        if let Some(stderr) = child.stderr.take() {
            std::thread::spawn(move || {
                let reader = BufReader::new(stderr);
                for line in reader.lines() {
                    match line {
                        Ok(l) => debug!("sidecar: {}", l),
                        Err(_) => break,
                    }
                }
            });
        }

        let mut reader = BufReader::new(stdout);

        // Wait for the "ready" message (blocking — sidecar loads models then signals)
        info!("Waiting for sidecar ready signal...");
        let mut ready_line = String::new();
        let bytes = reader.read_line(&mut ready_line)
            .map_err(|e| AppError::Transcription(format!("Failed to read sidecar ready signal: {}", e)))?;
        if bytes == 0 {
            return Err(AppError::Transcription("Sidecar exited before sending ready signal".to_string()));
        }
        let ready_line = ready_line.trim().to_string();

        let msg: SidecarResult = serde_json::from_str(&ready_line)
            .map_err(|e| AppError::Transcription(format!("Invalid ready message: {} - raw: {}", e, ready_line)))?;

        if msg.status.as_deref() != Some("ready") {
            return Err(AppError::Transcription(format!(
                "Expected ready status, got: {}", ready_line
            )));
        }

        info!("Sidecar is ready (PID: {})", child.id());

        self.child = Some(child);
        self.stdin = Some(stdin);
        self.stdout_reader = Some(reader);

        Ok(())
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

    /// Send a request to the sidecar and read the response
    fn call_sidecar(
        &mut self,
        audio_path: &Path,
        language: TranscriptionLanguage,
        config: &DecodingConfig,
    ) -> Result<SidecarResult> {
        let language_str = match language {
            TranscriptionLanguage::Auto => "auto",
            TranscriptionLanguage::French => "french",
            TranscriptionLanguage::English => "english",
        };

        let request = SidecarRequest {
            audio_path: audio_path.to_string_lossy().to_string(),
            language: language_str.to_string(),
            beam_width: config.beam_width,
            temperature: config.temperature,
            blank_penalty: config.blank_penalty,
            command: None,
        };

        let request_json = serde_json::to_string(&request)
            .map_err(|e| AppError::Transcription(format!("Failed to serialize request: {}", e)))?;

        debug!("Sending request to sidecar: {}", request_json);

        // Write request to stdin
        let stdin = self.stdin.as_mut()
            .ok_or_else(|| AppError::Transcription("Sidecar stdin not available".to_string()))?;

        writeln!(stdin, "{}", request_json)
            .map_err(|e| AppError::Transcription(format!("Failed to write to sidecar stdin: {}", e)))?;
        stdin.flush()
            .map_err(|e| AppError::Transcription(format!("Failed to flush sidecar stdin: {}", e)))?;

        // Read response from stdout
        let reader = self.stdout_reader.as_mut()
            .ok_or_else(|| AppError::Transcription("Sidecar stdout not available".to_string()))?;

        let mut line = String::new();
        let bytes_read = reader.read_line(&mut line)
            .map_err(|e| AppError::Transcription(format!("Failed to read sidecar response: {}", e)))?;

        if bytes_read == 0 {
            return Err(AppError::Transcription("Sidecar closed stdout (EOF) — process may have crashed".to_string()));
        }

        let trimmed = line.trim();
        debug!("Sidecar response: {}", trimmed);

        serde_json::from_str::<SidecarResult>(trimmed)
            .map_err(|e| AppError::Transcription(format!("Failed to parse sidecar response: {} - raw: {}", e, trimmed)))
    }

    /// Check if the sidecar process is still alive
    fn is_sidecar_alive(&mut self) -> bool {
        if let Some(child) = self.child.as_mut() {
            matches!(child.try_wait(), Ok(None))
        } else {
            false
        }
    }

    /// Kill the sidecar and clear handles
    fn kill_sidecar(&mut self) {
        // Drop stdin first to signal EOF
        self.stdin.take();
        self.stdout_reader.take();

        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }

    /// Send a request, respawning the sidecar if it has crashed
    fn call_sidecar_with_respawn(
        &mut self,
        audio_path: &Path,
        language: TranscriptionLanguage,
        config: &DecodingConfig,
    ) -> Result<SidecarResult> {
        // Try the call
        match self.call_sidecar(audio_path, language, config) {
            Ok(result) => Ok(result),
            Err(e) => {
                // Check if sidecar died
                if !self.is_sidecar_alive() {
                    warn!("Sidecar crashed, attempting respawn: {}", e);
                    self.kill_sidecar();
                    self.spawn_sidecar()?;
                    // Retry once after respawn
                    self.call_sidecar(audio_path, language, config)
                } else {
                    Err(e)
                }
            }
        }
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
        self.child.is_some() && self.stdin.is_some()
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

        // Spawn the persistent sidecar and wait for it to load models
        self.spawn_sidecar()?;

        info!("CoreML persistent sidecar engine ready");
        Ok(())
    }

    fn run_inference(
        &mut self,
        samples: &[f32],
        language: TranscriptionLanguage,
        config: &DecodingConfig,
    ) -> Result<String> {
        info!(
            "Starting CoreML sidecar inference on {} samples ({:.2}s), language={:?}, beam_width={}, temp={:.2}, blank_penalty={:.1}",
            samples.len(),
            samples.len() as f32 / 16000.0,
            language,
            config.beam_width,
            config.temperature,
            config.blank_penalty
        );

        // Write audio to temp file
        let temp_wav = self.write_temp_wav(samples)?;

        // Send request to persistent sidecar (with auto-respawn)
        let result = self.call_sidecar_with_respawn(&temp_wav, language, config);

        // Clean up temp file
        if let Err(e) = std::fs::remove_file(&temp_wav) {
            warn!("Failed to remove temp WAV: {}", e);
        }

        let result = result?;

        // Check for error in response
        if let Some(error) = result.error {
            return Err(AppError::Transcription(error));
        }

        let text = result.text.unwrap_or_default();
        let confidence = result.confidence.unwrap_or(0.0);
        let time_ms = result.processing_time_ms.unwrap_or(0);

        info!(
            "CoreML transcription: confidence={:.2}%, time={}ms",
            confidence * 100.0,
            time_ms
        );

        Ok(text)
    }
}

impl Drop for CoreMLEngine {
    fn drop(&mut self) {
        if self.child.is_some() {
            info!("Shutting down CoreML sidecar");

            // Try to send quit command gracefully
            if let Some(stdin) = self.stdin.as_mut() {
                let _ = writeln!(stdin, r#"{{"command":"quit"}}"#);
                let _ = stdin.flush();
            }

            // Give it a moment then force-kill
            self.stdin.take();
            self.stdout_reader.take();

            if let Some(mut child) = self.child.take() {
                match child.try_wait() {
                    Ok(Some(_)) => {} // Already exited
                    _ => {
                        // Wait briefly, then kill
                        std::thread::sleep(Duration::from_millis(500));
                        match child.try_wait() {
                            Ok(Some(_)) => {}
                            _ => {
                                warn!("Sidecar didn't exit gracefully, killing");
                                let _ = child.kill();
                                let _ = child.wait();
                            }
                        }
                    }
                }
            }
        }
    }
}
