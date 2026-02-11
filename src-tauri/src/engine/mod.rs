pub mod config;
#[cfg(target_os = "macos")]
pub mod coreml;
pub mod decoder;
pub mod parakeet;

use crate::error::Result;
use crate::storage::{Segment, Transcription};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::Instant;
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

/// Filter out hallucinations at the start and end of chunk transcriptions.
/// Leading: spurious punctuation or short nonsense words from silence.
/// Trailing: English sentences, short fragments after final punctuation, repetition loops.
pub fn filter_chunk_hallucinations(text: &str) -> String {
    let text = text.trim();
    if text.is_empty() {
        return String::new();
    }

    // === Leading filters ===
    let cleaned = RE_LEADING_PUNCT.replace(text, "");
    let cleaned = RE_SHORT_WORD.replace(&cleaned, "");
    let cleaned = RE_MULTI_HALLUC.replace(&cleaned, "");
    let mut result = cleaned.trim().to_string();

    // === Repetition filter (n-gram loop detection) ===
    result = filter_repetitions(&result);

    // === Trailing hallucination filter ===
    result = filter_trailing_hallucinations(&result);

    if result != text {
        debug!("Filtered hallucination: '{}' -> '{}'", text, result);
    }

    result
}

/// Detect and truncate n-gram repetition loops.
/// e.g. "de la fin de la fin de la fin de la fin" → "de la fin"
fn filter_repetitions(text: &str) -> String {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.len() < 6 {
        return text.to_string();
    }

    // Check n-grams of length 2..6 words
    for n in 2..=6 {
        if words.len() < n * 3 {
            continue;
        }
        // Scan from the end backwards: find where a repeating pattern starts
        // Check if the last 3*n words contain the same n-gram repeated 3+ times
        for start in 0..words.len().saturating_sub(n * 3) {
            let ngram = &words[start..start + n];
            let mut repeats = 1;
            let mut pos = start + n;
            while pos + n <= words.len() {
                if &words[pos..pos + n] == ngram {
                    repeats += 1;
                    pos += n;
                } else {
                    break;
                }
            }
            if repeats >= 3 {
                // Keep everything before the repetition + one instance of the n-gram
                let keep_until = start + n;
                let truncated = words[..keep_until].join(" ");
                // Append whatever comes after the repetition block
                let after_reps = start + n * repeats;
                let suffix = if after_reps < words.len() {
                    format!(" {}", words[after_reps..].join(" "))
                } else {
                    String::new()
                };
                let final_text = format!("{}{}", truncated, suffix);
                debug!(
                    "Repetition filter: '{}' repeated {}x, truncated",
                    ngram.join(" "),
                    repeats
                );
                // Recurse in case there are multiple repetition blocks
                return filter_repetitions(&final_text);
            }
        }
    }

    text.to_string()
}

/// Remove trailing hallucination fragments after the last sentence.
/// Language-agnostic: only removes very short residual fragments (< 15 chars)
/// that appear after the last sentence-ending punctuation.
/// e.g. "...en segments. Ye" → "...en segments."
fn filter_trailing_hallucinations(text: &str) -> String {
    let text = text.trim();
    if text.is_empty() {
        return String::new();
    }

    // Find the last sentence-ending punctuation (. ! ?)
    let last_sentence_end = text.rfind(|c: char| c == '.' || c == '!' || c == '?');

    if let Some(pos) = last_sentence_end {
        let after = text[pos + 1..].trim();
        // Only remove very short trailing fragments (< 15 chars, no sentence punct)
        // Catches "Ye", "Yeah", "Ok", stray words — but not legitimate continuations
        if !after.is_empty()
            && after.len() < 15
            && !after.contains('.')
            && !after.contains('!')
            && !after.contains('?')
        {
            debug!("Trailing fragment removed: '{}'", after);
            return text[..=pos].trim().to_string();
        }
    }

    text.to_string()
}

/// Maximum audio samples per chunk (15 seconds at 16kHz)
pub const MAX_AUDIO_SAMPLES: usize = 240000;

pub use config::DecodingConfig;
#[cfg(target_os = "macos")]
pub use coreml::CoreMLEngine;
pub use parakeet::ParakeetEngine;

/// Language selection for transcription
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TranscriptionLanguage {
    /// Auto-detect language (default)
    #[default]
    Auto,
    /// Force French
    French,
    /// Force English
    English,
}

impl TranscriptionLanguage {
    /// Get the token ID to inject for this language
    /// Returns None for Auto (let the model decide)
    pub fn token_id(&self) -> Option<i64> {
        match self {
            TranscriptionLanguage::Auto => None,
            TranscriptionLanguage::French => Some(71),  // <|fr|>
            TranscriptionLanguage::English => Some(64), // <|en|>
        }
    }

    /// Get display name
    pub fn display_name(&self) -> &'static str {
        match self {
            TranscriptionLanguage::Auto => "Auto",
            TranscriptionLanguage::French => "Français",
            TranscriptionLanguage::English => "English",
        }
    }

    /// Get ISO language code for storage
    pub fn code(&self) -> &'static str {
        match self {
            TranscriptionLanguage::Auto => "auto",
            TranscriptionLanguage::French => "fr",
            TranscriptionLanguage::English => "en",
        }
    }
}

/// Available inference backends
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum EngineBackend {
    /// OpenVINO backend (FluidInference model)
    #[default]
    OpenVINO,
    /// CoreML backend (Apple platforms only)
    #[cfg(target_os = "macos")]
    CoreML,
}

impl EngineBackend {
    /// Get the model subdirectory name for this backend
    pub fn model_subdir(&self) -> &'static str {
        match self {
            EngineBackend::OpenVINO => "openvino",
            #[cfg(target_os = "macos")]
            EngineBackend::CoreML => "coreml",
        }
    }

    /// Get display name
    pub fn display_name(&self) -> &'static str {
        match self {
            EngineBackend::OpenVINO => "OpenVINO",
            #[cfg(target_os = "macos")]
            EngineBackend::CoreML => "CoreML",
        }
    }
}

/// Trait for ASR inference engines
///
/// This allows swapping between different backends (OpenVINO, CoreML)
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
        &mut self,
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
            #[cfg(target_os = "macos")]
            EngineBackend::CoreML => Box::new(CoreMLEngine::new()),
        };

        new_engine.load_model(model_dir)?;
        self.engine = new_engine;
        self.backend = backend;

        info!("Switched to {} backend successfully", backend.display_name());
        Ok(())
    }

    /// Transcribe a chunk of audio and return just the text.
    /// Used for incremental streaming during recording.
    pub fn transcribe_chunk(
        &mut self,
        samples: &[f32],
        language: TranscriptionLanguage,
        config: &DecodingConfig,
    ) -> Result<String> {
        if !self.is_loaded() {
            return Ok(String::new());
        }
        let start = Instant::now();
        let text = self.engine.run_inference(samples, language, config)?;
        let elapsed = start.elapsed().as_millis();
        info!(
            "Chunk inference: {} samples ({:.1}s) -> {} chars in {}ms",
            samples.len(),
            samples.len() as f64 / 16000.0,
            text.len(),
            elapsed
        );
        Ok(text)
    }

    /// Transcribe audio samples (16kHz mono f32)
    pub fn transcribe(
        &mut self,
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

        let start = Instant::now();
        match self.engine.run_inference(samples, language, &config) {
            Ok(text) => {
                let processing_time_ms = start.elapsed().as_millis() as i64;
                info!("Inference completed in {} ms", processing_time_ms);

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
                    language: language.code().to_string(),
                    segments,
                    raw_text: text,
                    edited_text: None,
                    is_edited: false,
                    processing_time_ms,
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
            processing_time_ms: 0,
        })
    }
}
