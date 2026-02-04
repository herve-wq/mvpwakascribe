pub mod config;
#[cfg(target_os = "macos")]
pub mod coreml;
pub mod decoder;
pub mod mel;
pub mod merger; // Kept for potential future use (LCS-based merge)
pub mod onnxruntime;
pub mod parakeet;

use crate::error::Result;
use crate::storage::{Segment, Transcription};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tracing::{debug, info, warn};
use uuid::Uuid;

// ============================================================================
// Common utilities shared between backends
// ============================================================================

// Pre-compiled regexes for hallucination filtering
static RE_LEADING_PUNCT: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(?:[\.\,\-\;\:\!\?]+\s*)+").unwrap()
});
static RE_SHORT_WORD: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^[A-Za-z0-9]{1,4}[\.\,\-\;\:]\s*").unwrap()
});
static RE_MULTI_HALLUC: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(?:[\.\,\-\;\:\!\?]?\s*[A-Za-z0-9]{1,4}[\.\,\-\;\:]\s*)+").unwrap()
});

/// Filter out hallucinations at the start of chunk transcriptions
/// These are typically short spurious words or punctuation that the model
/// generates when starting from silence.
pub fn filter_chunk_hallucinations(text: &str) -> String {
    let text = text.trim();
    if text.is_empty() {
        return String::new();
    }

    // Common hallucination patterns at chunk start:
    // - Single punctuation marks (". ", ", ", "- ")
    // - Very short nonsense words followed by punctuation ("Ture.", "MDF-", "CIS.")
    // - Numbers alone at start ("260", "6.")

    // Pattern: Remove leading garbage (punctuation, short gibberish words < 4 chars followed by punctuation)
    let cleaned = RE_LEADING_PUNCT.replace(text, "");

    // Also remove short words (1-4 chars) at the very start if followed by punctuation
    let cleaned = RE_SHORT_WORD.replace(&cleaned, "");

    // Handle multiple short hallucinations chained (". Ture. Règle" -> "Règle")
    let cleaned = RE_MULTI_HALLUC.replace(&cleaned, "");

    let result = cleaned.trim().to_string();

    if result != text {
        debug!("Filtered hallucination: '{}' -> '{}'", text, result);
    }

    result
}

/// Maximum audio samples per chunk (15 seconds at 16kHz)
pub const MAX_AUDIO_SAMPLES: usize = 240000;

pub use config::DecodingConfig;
#[cfg(target_os = "macos")]
pub use coreml::CoreMLEngine;
pub use onnxruntime::OnnxRuntimeEngine;
pub use parakeet::{ParakeetEngine, TranscriptionLanguage};

// Re-export for use in commands

/// Available inference backends
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum EngineBackend {
    /// OpenVINO backend (FluidInference model)
    #[default]
    OpenVINO,
    /// ONNX Runtime backend (istupakov model)
    OnnxRuntime,
    /// CoreML backend (Apple platforms only)
    #[cfg(target_os = "macos")]
    CoreML,
}

impl EngineBackend {
    /// Get the model subdirectory name for this backend
    pub fn model_subdir(&self) -> &'static str {
        match self {
            EngineBackend::OpenVINO => "openvino",
            EngineBackend::OnnxRuntime => "onnxruntime",
            #[cfg(target_os = "macos")]
            EngineBackend::CoreML => "coreml",
        }
    }

    /// Get display name
    pub fn display_name(&self) -> &'static str {
        match self {
            EngineBackend::OpenVINO => "OpenVINO",
            EngineBackend::OnnxRuntime => "ONNX Runtime",
            #[cfg(target_os = "macos")]
            EngineBackend::CoreML => "CoreML",
        }
    }
}

/// Trait for ASR inference engines
///
/// This allows swapping between different backends (OpenVINO, ONNX Runtime)
/// while keeping a consistent interface.
pub trait ASREngine: Send + Sync {
    /// Get the engine name for logging
    fn name(&self) -> &str;

    /// Check if models are loaded and ready
    fn is_loaded(&self) -> bool;

    /// Load models from the given directory
    fn load_model(&mut self, model_dir: &Path) -> Result<()>;

    /// Run inference on audio samples
    ///
    /// # Arguments
    /// * `samples` - Audio samples (16kHz mono f32, normalized)
    /// * `language` - Target language for transcription
    /// * `config` - Decoding configuration (beam width, temperature, etc.)
    ///
    /// # Returns
    /// Transcribed text
    fn run_inference(
        &self,
        samples: &[f32],
        language: TranscriptionLanguage,
        config: &DecodingConfig,
    ) -> Result<String>;
}

/// Dynamic engine wrapper that can switch between backends at runtime
pub struct DynamicEngine {
    engine: Box<dyn ASREngine>,
    backend: EngineBackend,
}

impl DynamicEngine {
    /// Create a new dynamic engine with the specified backend
    pub fn new(backend: EngineBackend) -> Self {
        let engine: Box<dyn ASREngine> = match backend {
            EngineBackend::OpenVINO => Box::new(ParakeetEngine::new()),
            EngineBackend::OnnxRuntime => Box::new(OnnxRuntimeEngine::new()),
            #[cfg(target_os = "macos")]
            EngineBackend::CoreML => Box::new(CoreMLEngine::new()),
        };
        Self { engine, backend }
    }

    /// Get the current backend type
    pub fn backend(&self) -> EngineBackend {
        self.backend
    }

    /// Get the engine name
    pub fn name(&self) -> &str {
        self.engine.name()
    }

    /// Check if the engine is loaded
    pub fn is_loaded(&self) -> bool {
        self.engine.is_loaded()
    }

    /// Load the model from the given directory
    pub fn load_model(&mut self, model_dir: &Path) -> Result<()> {
        self.engine.load_model(model_dir)
    }

    /// Switch to a different backend (requires reloading model)
    pub fn switch_backend(&mut self, backend: EngineBackend, model_dir: &Path) -> Result<()> {
        if backend == self.backend {
            return Ok(());
        }

        info!("Switching engine from {} to {}", self.backend.display_name(), backend.display_name());

        let mut new_engine: Box<dyn ASREngine> = match backend {
            EngineBackend::OpenVINO => Box::new(ParakeetEngine::new()),
            EngineBackend::OnnxRuntime => Box::new(OnnxRuntimeEngine::new()),
            #[cfg(target_os = "macos")]
            EngineBackend::CoreML => Box::new(CoreMLEngine::new()),
        };

        new_engine.load_model(model_dir)?;
        self.engine = new_engine;
        self.backend = backend;

        info!("Switched to {} backend successfully", backend.display_name());
        Ok(())
    }

    /// Transcribe audio samples (16kHz mono f32)
    pub fn transcribe(
        &self,
        samples: &[f32],
        source_type: &str,
        source_name: Option<String>,
        language: TranscriptionLanguage,
        decoding_config: Option<DecodingConfig>,
    ) -> Result<Transcription> {
        let duration_ms = (samples.len() as f64 / 16000.0 * 1000.0) as i64;
        let config = decoding_config.unwrap_or_default();

        if !self.is_loaded() {
            info!("Engine not loaded, returning mock transcription");
            return Self::mock_transcribe(samples, source_type, source_name);
        }

        info!(
            "Transcribing {} samples ({} ms) with {}, language: {:?}, beam_width: {}, temperature: {:.2}",
            samples.len(),
            duration_ms,
            self.name(),
            language,
            config.beam_width,
            config.temperature
        );

        match self.engine.run_inference(samples, language, &config) {
            Ok(text) => {
                let now = chrono::Utc::now().to_rfc3339();
                let segments = vec![Segment {
                    id: Uuid::new_v4().to_string(),
                    start_ms: 0,
                    end_ms: duration_ms,
                    text: text.clone(),
                    confidence: 0.95,
                }];

                Ok(Transcription {
                    id: Uuid::new_v4().to_string(),
                    created_at: now.clone(),
                    updated_at: now,
                    source_type: source_type.to_string(),
                    source_name,
                    duration_ms,
                    language: "fr".to_string(),
                    segments,
                    raw_text: text,
                    edited_text: None,
                    is_edited: false,
                })
            }
            Err(e) => {
                warn!("Inference failed: {}. Falling back to mock transcription.", e);
                Self::mock_transcribe(samples, source_type, source_name)
            }
        }
    }

    /// Generate mock transcription when model isn't loaded
    fn mock_transcribe(
        samples: &[f32],
        source_type: &str,
        source_name: Option<String>,
    ) -> Result<Transcription> {
        let duration_ms = (samples.len() as f64 / 16000.0 * 1000.0) as i64;
        let now = chrono::Utc::now().to_rfc3339();

        let mock_text = "[Moteur non charge - transcription simulee]".to_string();

        Ok(Transcription {
            id: Uuid::new_v4().to_string(),
            created_at: now.clone(),
            updated_at: now,
            source_type: source_type.to_string(),
            source_name,
            duration_ms,
            language: "fr".to_string(),
            segments: vec![Segment {
                id: Uuid::new_v4().to_string(),
                start_ms: 0,
                end_ms: duration_ms,
                text: mock_text.clone(),
                confidence: 0.0,
            }],
            raw_text: mock_text,
            edited_text: None,
            is_edited: false,
        })
    }
}
