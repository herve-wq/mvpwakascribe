//! ONNX Runtime backend for Parakeet TDT inference
//!
//! This backend uses the istupakov/parakeet-tdt-0.6b-v3-onnx model:
//! - nemo128.onnx: Mel spectrogram
//! - encoder-model.int8.onnx: FastConformer encoder
//! - decoder_joint-model.onnx: Combined decoder + joint network

use crate::audio::{split_audio_smart, SmartChunkConfig};
use crate::engine::config::DecodingConfig;
use crate::engine::decoder::{TDTDecoder, Vocabulary};
use crate::engine::{filter_chunk_hallucinations, ASREngine, MAX_AUDIO_SAMPLES};
use crate::engine::TranscriptionLanguage;
use crate::error::{AppError, Result};
use ort::session::{builder::GraphOptimizationLevel, Session};
use ort::value::Tensor;
use std::path::Path;
use std::sync::Mutex;
use tracing::{debug, info, warn};

/// Token spécial pour le blank (pas de sortie)
const BLANK_TOKEN: u32 = 8192;

/// Nombre de classes de durée TDT (1, 2, 3, 4, 5 frames)
const NUM_DURATION_CLASSES: usize = 5;

/// Taille totale du vocabulaire (tokens + blank)
const VOCAB_SIZE: usize = 8193;

/// Dimension cachée du décodeur LSTM
const DECODER_HIDDEN_DIM: usize = 640;

/// Nombre de couches LSTM
const DECODER_NUM_LAYERS: usize = 2;

/// Encoder output dimension
const ENCODER_OUTPUT_DIM: usize = 1024;

/// Mel features dimension
const MEL_FEATURES: usize = 128;

/// LSTM states for decoder
struct LSTMStates {
    h: Vec<f32>, // [2, 1, 640] flattened
    c: Vec<f32>, // [2, 1, 640] flattened
}

impl LSTMStates {
    fn zeros() -> Self {
        let size = DECODER_NUM_LAYERS * DECODER_HIDDEN_DIM;
        Self {
            h: vec![0.0; size],
            c: vec![0.0; size],
        }
    }
}

/// Beam hypothesis for beam search decoding
#[derive(Clone)]
struct BeamHypothesis {
    /// Emitted tokens so far
    tokens: Vec<u32>,
    /// Cumulative log probability score
    score: f32,
    /// LSTM hidden state
    h_state: Vec<f32>,
    /// LSTM cell state
    c_state: Vec<f32>,
    /// Last emitted token (or BLANK)
    last_token: i32,
    /// Current time position in encoder output
    current_time: usize,
}

/// ONNX Runtime engine for Parakeet TDT
pub struct OnnxRuntimeEngine {
    mel_session: Option<Mutex<Session>>,
    encoder_session: Option<Mutex<Session>>,
    decoder_joint_session: Option<Mutex<Session>>,
    tdt_decoder: Option<TDTDecoder>,
}

// Implement Send + Sync
unsafe impl Send for OnnxRuntimeEngine {}
unsafe impl Sync for OnnxRuntimeEngine {}

impl OnnxRuntimeEngine {
    pub fn new() -> Self {
        Self {
            mel_session: None,
            encoder_session: None,
            decoder_joint_session: None,
            tdt_decoder: None,
        }
    }

    /// Compute mel spectrogram from audio
    fn compute_mel(&self, audio: &[f32]) -> Result<(Vec<f32>, usize, i64)> {
        let session = self.mel_session.as_ref()
            .ok_or_else(|| AppError::Transcription("Mel session not loaded".to_string()))?;
        let mut session = session.lock().unwrap();

        let audio_len = audio.len() as i64;

        // Prepare inputs
        // waveforms: [1, N]
        let waveforms = Tensor::from_array(([1usize, audio.len()], audio.to_vec()))
            .map_err(|e| AppError::Transcription(format!("Failed to create waveforms tensor: {}", e)))?;

        // waveforms_lens: [1]
        let waveforms_lens = Tensor::from_array(([1usize], vec![audio_len]))
            .map_err(|e| AppError::Transcription(format!("Failed to create waveforms_lens tensor: {}", e)))?;

        // Run inference
        let outputs = session.run(ort::inputs![
            "waveforms" => waveforms,
            "waveforms_lens" => waveforms_lens,
        ]).map_err(|e| AppError::Transcription(format!("Mel inference failed: {}", e)))?;

        // Get outputs
        // features: [1, 128, T]
        let (features_shape, features_data) = outputs["features"]
            .try_extract_tensor::<f32>()
            .map_err(|e| AppError::Transcription(format!("Failed to extract features: {}", e)))?;

        let (_, features_lens_data) = outputs["features_lens"]
            .try_extract_tensor::<i64>()
            .map_err(|e| AppError::Transcription(format!("Failed to extract features_lens: {}", e)))?;

        let t = features_shape[2] as usize;
        let features_len = features_lens_data[0];

        debug!("Mel output: {} frames, features_len={}", t, features_len);
        Ok((features_data.to_vec(), t, features_len))
    }

    /// Run encoder on mel features
    fn run_encoder(&self, mel_data: &[f32], mel_time: usize, mel_len: i64) -> Result<(Vec<f32>, usize, usize)> {
        let session = self.encoder_session.as_ref()
            .ok_or_else(|| AppError::Transcription("Encoder session not loaded".to_string()))?;
        let mut session = session.lock().unwrap();

        // Prepare inputs
        // audio_signal: [1, 128, T]
        let audio_signal = Tensor::from_array(([1usize, MEL_FEATURES, mel_time], mel_data.to_vec()))
            .map_err(|e| AppError::Transcription(format!("Failed to create audio_signal tensor: {}", e)))?;

        // length: [1]
        let length = Tensor::from_array(([1usize], vec![mel_len]))
            .map_err(|e| AppError::Transcription(format!("Failed to create length tensor: {}", e)))?;

        // Run inference
        let outputs = session.run(ort::inputs![
            "audio_signal" => audio_signal,
            "length" => length,
        ]).map_err(|e| AppError::Transcription(format!("Encoder inference failed: {}", e)))?;

        // Get outputs
        // outputs: [1, 1024, T']
        let (enc_shape, encoder_data) = outputs["outputs"]
            .try_extract_tensor::<f32>()
            .map_err(|e| AppError::Transcription(format!("Failed to extract encoder outputs: {}", e)))?;

        let (_, encoded_lens_data) = outputs["encoded_lengths"]
            .try_extract_tensor::<i64>()
            .map_err(|e| AppError::Transcription(format!("Failed to extract encoded_lengths: {}", e)))?;

        let t_enc = enc_shape[2] as usize;
        let valid_time = encoded_lens_data[0] as usize;

        debug!("Encoder output: {} time steps, valid={}", t_enc, valid_time);
        Ok((encoder_data.to_vec(), t_enc, valid_time))
    }

    /// Run decoder+joint on a single encoder frame
    fn run_decoder_joint(
        &self,
        encoder_data: &[f32],
        encoder_time: usize,
        t: usize,
        target: i32,
        states: &mut LSTMStates,
    ) -> Result<Vec<f32>> {
        let session = self.decoder_joint_session.as_ref()
            .ok_or_else(|| AppError::Transcription("Decoder+Joint session not loaded".to_string()))?;
        let mut session = session.lock().unwrap();

        // Extract single encoder frame: data is [1, 1024, T] in row-major
        // We need frame at time t: indices [0, :, t]
        let mut encoder_frame = vec![0.0f32; ENCODER_OUTPUT_DIM];
        for d in 0..ENCODER_OUTPUT_DIM {
            // Index in [1, 1024, T] flattened = d * T + t
            let idx = d * encoder_time + t;
            encoder_frame[d] = encoder_data[idx];
        }

        // Prepare inputs
        // encoder_outputs: [1, 1024, 1]
        let encoder_outputs = Tensor::from_array(([1usize, ENCODER_OUTPUT_DIM, 1usize], encoder_frame))
            .map_err(|e| AppError::Transcription(format!("Failed to create encoder_outputs tensor: {}", e)))?;

        // targets: [1, 1]
        let targets = Tensor::from_array(([1usize, 1usize], vec![target]))
            .map_err(|e| AppError::Transcription(format!("Failed to create targets tensor: {}", e)))?;

        // target_length: [1]
        let target_length = Tensor::from_array(([1usize], vec![1i32]))
            .map_err(|e| AppError::Transcription(format!("Failed to create target_length tensor: {}", e)))?;

        // input_states_1: [2, 1, 640]
        let input_states_1 = Tensor::from_array(([DECODER_NUM_LAYERS, 1usize, DECODER_HIDDEN_DIM], states.h.clone()))
            .map_err(|e| AppError::Transcription(format!("Failed to create input_states_1 tensor: {}", e)))?;

        // input_states_2: [2, 1, 640]
        let input_states_2 = Tensor::from_array(([DECODER_NUM_LAYERS, 1usize, DECODER_HIDDEN_DIM], states.c.clone()))
            .map_err(|e| AppError::Transcription(format!("Failed to create input_states_2 tensor: {}", e)))?;

        // Run inference
        let outputs = session.run(ort::inputs![
            "encoder_outputs" => encoder_outputs,
            "targets" => targets,
            "target_length" => target_length,
            "input_states_1" => input_states_1,
            "input_states_2" => input_states_2,
        ]).map_err(|e| AppError::Transcription(format!("Decoder+Joint inference failed: {}", e)))?;

        // Get outputs
        // outputs: [1, 1, 1, 8198]
        let (_, logits_data) = outputs["outputs"]
            .try_extract_tensor::<f32>()
            .map_err(|e| AppError::Transcription(format!("Failed to extract logits: {}", e)))?;

        // Update states
        let (_, new_h_data) = outputs["output_states_1"]
            .try_extract_tensor::<f32>()
            .map_err(|e| AppError::Transcription(format!("Failed to extract output_states_1: {}", e)))?;

        let (_, new_c_data) = outputs["output_states_2"]
            .try_extract_tensor::<f32>()
            .map_err(|e| AppError::Transcription(format!("Failed to extract output_states_2: {}", e)))?;

        // Copy new states
        states.h = new_h_data.to_vec();
        states.c = new_c_data.to_vec();

        // Extract logits
        Ok(logits_data.to_vec())
    }

    /// Decode TDT output (token + duration) from joint logits
    fn decode_tdt_output(&self, logits: &[f32], config: &DecodingConfig) -> (u32, usize) {
        // Split logits into token and duration parts
        let token_logits = &logits[..VOCAB_SIZE];
        let duration_logits = &logits[VOCAB_SIZE..VOCAB_SIZE + NUM_DURATION_CLASSES];

        // Apply temperature scaling
        let scaled_token_logits: Vec<f32> = if config.temperature != 1.0 && config.temperature > 0.0 {
            token_logits.iter().map(|&l| l / config.temperature).collect()
        } else {
            token_logits.to_vec()
        };

        // Apply blank penalty
        let mut final_logits = scaled_token_logits;
        final_logits[BLANK_TOKEN as usize] -= config.blank_penalty;

        // Find best token (argmax)
        let (best_token, _) = final_logits
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .unwrap();

        // Find best duration (argmax)
        let (best_dur_idx, _) = duration_logits
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .unwrap();

        // Duration is 1-indexed (dur_idx 0 = 1 frame, dur_idx 4 = 5 frames)
        let duration = best_dur_idx + 1;

        (best_token as u32, duration)
    }

    /// TDT greedy decoding
    fn tdt_greedy_decode(
        &self,
        encoder_data: &[f32],
        encoder_time: usize,
        valid_time: usize,
        _language: TranscriptionLanguage,
        config: &DecodingConfig,
    ) -> Result<Vec<u32>> {
        let mut states = LSTMStates::zeros();
        let mut tokens = Vec::new();
        let mut t = 0;
        let mut iterations = 0;
        const MAX_ITERATIONS: usize = 1000;

        info!(
            "TDT decode config: beam=1, temp={:.2}, blank_penalty={:.1}",
            config.temperature, config.blank_penalty
        );

        while t < valid_time && iterations < MAX_ITERATIONS {
            iterations += 1;

            // Get last token (or blank for start)
            let last_token = tokens.last().copied().unwrap_or(BLANK_TOKEN) as i32;

            // Run decoder+joint
            let logits = self.run_decoder_joint(encoder_data, encoder_time, t, last_token, &mut states)?;

            // Decode token and duration
            let (token, duration) = self.decode_tdt_output(&logits, config);

            if token != BLANK_TOKEN {
                tokens.push(token);
            }

            // Advance time by duration
            t += duration;

            if iterations <= 5 {
                debug!(
                    "t={}, token={}, dur={}, total_tokens={}",
                    t, token, duration, tokens.len()
                );
            }
        }

        info!(
            "TDT decoded {} tokens in {} iterations",
            tokens.len(),
            iterations
        );

        Ok(tokens)
    }

    /// Convert tokens to text
    fn tokens_to_text(&self, tokens: &[u32]) -> String {
        let decoder = self.tdt_decoder.as_ref();
        if decoder.is_none() {
            return String::new();
        }

        let decoder = decoder.unwrap();
        let mut text = String::new();

        for &token in tokens {
            if token == BLANK_TOKEN || token as usize >= VOCAB_SIZE {
                continue;
            }

            let token_text = decoder.decode_single(token as usize);
            text.push_str(&token_text);
        }

        text.trim().to_string()
    }
}

impl Default for OnnxRuntimeEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl ASREngine for OnnxRuntimeEngine {
    fn name(&self) -> &str {
        "ONNX Runtime"
    }

    fn is_loaded(&self) -> bool {
        self.mel_session.is_some()
            && self.encoder_session.is_some()
            && self.decoder_joint_session.is_some()
            && self.tdt_decoder.is_some()
    }

    fn load_model(&mut self, model_dir: &Path) -> Result<()> {
        info!("Loading ONNX Runtime models from {:?}", model_dir);

        // Initialize ONNX Runtime (commit() returns bool in ort 2.0)
        let _ = ort::init()
            .with_name("WakaScribe")
            .commit();

        // Load vocabulary
        let vocab_path = model_dir.join("vocab.txt");
        if vocab_path.exists() {
            let vocab = Vocabulary::load_txt(&vocab_path)?;
            info!(
                "Loaded vocabulary with {} tokens from {:?}",
                vocab.vocab_size(),
                vocab_path
            );
            self.tdt_decoder = Some(TDTDecoder::new(vocab));
        } else {
            return Err(AppError::Transcription(format!(
                "Vocabulary file not found: {:?}",
                vocab_path
            )));
        }

        // Load mel spectrogram model
        info!("Loading mel spectrogram model (nemo128.onnx)...");
        let mel_path = model_dir.join("nemo128.onnx");
        let mel_session = Session::builder()
            .map_err(|e| AppError::Transcription(format!("Failed to create session builder: {}", e)))?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(|e| AppError::Transcription(format!("Failed to set optimization level: {}", e)))?
            .commit_from_file(&mel_path)
            .map_err(|e| AppError::Transcription(format!("Failed to load mel model: {}", e)))?;
        self.mel_session = Some(Mutex::new(mel_session));
        info!("Mel spectrogram model loaded");

        // Load encoder model (prefer int8 for speed)
        let encoder_path = if model_dir.join("encoder-model.int8.onnx").exists() {
            info!("Loading encoder model (encoder-model.int8.onnx)...");
            model_dir.join("encoder-model.int8.onnx")
        } else {
            info!("Loading encoder model (encoder-model.onnx)...");
            model_dir.join("encoder-model.onnx")
        };
        let encoder_session = Session::builder()
            .map_err(|e| AppError::Transcription(format!("Failed to create session builder: {}", e)))?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(|e| AppError::Transcription(format!("Failed to set optimization level: {}", e)))?
            .commit_from_file(&encoder_path)
            .map_err(|e| AppError::Transcription(format!("Failed to load encoder model: {}", e)))?;
        self.encoder_session = Some(Mutex::new(encoder_session));
        info!("Encoder model loaded");

        // Load decoder+joint model
        info!("Loading decoder+joint model (decoder_joint-model.onnx)...");
        let decoder_joint_path = model_dir.join("decoder_joint-model.onnx");
        let decoder_joint_session = Session::builder()
            .map_err(|e| AppError::Transcription(format!("Failed to create session builder: {}", e)))?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(|e| AppError::Transcription(format!("Failed to set optimization level: {}", e)))?
            .commit_from_file(&decoder_joint_path)
            .map_err(|e| AppError::Transcription(format!("Failed to load decoder_joint model: {}", e)))?;
        self.decoder_joint_session = Some(Mutex::new(decoder_joint_session));
        info!("Decoder+Joint model loaded");

        info!("All ONNX Runtime models loaded successfully");
        Ok(())
    }

    fn run_inference(
        &self,
        samples: &[f32],
        language: TranscriptionLanguage,
        config: &DecodingConfig,
    ) -> Result<String> {
        info!(
            "Starting ONNX Runtime TDT inference on {} samples ({:.2}s)",
            samples.len(),
            samples.len() as f32 / 16000.0
        );

        // Check if audio needs chunking
        if samples.len() > MAX_AUDIO_SAMPLES {
            info!(
                "Audio too long ({} samples = {:.1}s), using chunked transcription",
                samples.len(),
                samples.len() as f32 / 16000.0
            );
            return self.run_chunked_inference(samples, language, config);
        }

        // Single chunk inference
        self.run_single_inference(samples, language, config)
    }
}

// Additional methods for OnnxRuntimeEngine (outside impl ASREngine)
impl OnnxRuntimeEngine {
    /// Run inference on a single chunk (max 15s)
    fn run_single_inference(
        &self,
        audio: &[f32],
        language: TranscriptionLanguage,
        config: &DecodingConfig,
    ) -> Result<String> {
        // Limit to max audio samples
        let audio = if audio.len() > MAX_AUDIO_SAMPLES {
            &audio[..MAX_AUDIO_SAMPLES]
        } else {
            audio
        };

        // Step 1: Compute mel spectrogram
        debug!("Computing mel spectrogram...");
        let (mel_data, mel_time, mel_len) = self.compute_mel(audio)?;
        debug!("Mel spectrogram: {} frames, mel_len={}", mel_time, mel_len);

        // Step 2: Run encoder
        debug!("Running encoder...");
        let (encoder_data, encoder_time, valid_time) = self.run_encoder(&mel_data, mel_time, mel_len)?;
        debug!(
            "Encoder output: {} time steps, valid={}",
            encoder_time, valid_time
        );

        // Step 3: TDT decode (greedy or beam search based on config)
        let tokens = if config.beam_width <= 1 {
            debug!("Running TDT greedy decode...");
            self.tdt_greedy_decode(&encoder_data, encoder_time, valid_time, language, config)?
        } else {
            debug!("Running TDT beam search (beam_width={})...", config.beam_width);
            self.tdt_beam_decode(&encoder_data, encoder_time, valid_time, language, config)?
        };
        debug!("Decoded {} tokens", tokens.len());

        // Step 4: Convert to text
        let text = self.tokens_to_text(&tokens);

        Ok(text)
    }

    /// Run chunked inference for long audio using VAD-based smart chunking
    fn run_chunked_inference(
        &self,
        audio: &[f32],
        language: TranscriptionLanguage,
        config: &DecodingConfig,
    ) -> Result<String> {
        // Use smart VAD-based chunking (cuts at silence points)
        let chunk_config = SmartChunkConfig::default(); // 8-14s, cuts at silence
        let chunks = split_audio_smart(audio, &chunk_config);

        info!(
            "Processing {} chunks for {:.1}s audio (VAD-based smart chunking)",
            chunks.len(),
            audio.len() as f32 / 16000.0
        );

        let mut transcriptions: Vec<String> = Vec::new();

        for (i, chunk) in chunks.iter().enumerate() {
            let chunk_duration = chunk.samples.len() as f32 / 16000.0;
            info!(
                "Processing chunk {}/{} ({:.1}s - {:.1}s, duration={:.1}s)",
                i + 1,
                chunks.len(),
                chunk.start_ms as f32 / 1000.0,
                chunk.end_ms as f32 / 1000.0,
                chunk_duration
            );

            match self.run_single_inference(&chunk.samples, language, config) {
                Ok(text) => {
                    let raw_text = text.trim().to_string();
                    // Filter hallucinations at chunk start
                    let text = filter_chunk_hallucinations(&raw_text);
                    if !text.is_empty() {
                        if text != raw_text {
                            info!("Chunk {} transcription (filtered): '{}' -> '{}'", i + 1, raw_text, text);
                        } else {
                            info!("Chunk {} transcription: '{}'", i + 1, text);
                        }
                        transcriptions.push(text);
                    } else {
                        debug!("Chunk {} produced empty transcription after filtering (silence?)", i + 1);
                    }
                }
                Err(e) => {
                    warn!("Chunk {} transcription failed: {}", i + 1, e);
                    // Continue with other chunks
                }
            }
        }

        if transcriptions.is_empty() {
            return Err(AppError::Transcription(
                "All chunks failed to transcribe".to_string(),
            ));
        }

        // Simple concatenation - no complex merge needed since we cut at silence
        let merged_text = transcriptions.join(" ");

        info!("Final transcription ({} chunks): '{}'", transcriptions.len(), merged_text);
        Ok(merged_text)
    }

    /// TDT beam search decoding
    fn tdt_beam_decode(
        &self,
        encoder_data: &[f32],
        encoder_time: usize,
        valid_time: usize,
        _language: TranscriptionLanguage,
        config: &DecodingConfig,
    ) -> Result<Vec<u32>> {
        let beam_width = config.beam_width.max(1);
        let temperature = config.temperature;

        info!(
            "Starting beam search decode: beam_width={}, temp={:.2}, blank_penalty={:.1}",
            beam_width, temperature, config.blank_penalty
        );

        // Initialize beams with a single hypothesis
        let mut beams: Vec<BeamHypothesis> = vec![BeamHypothesis {
            tokens: Vec::new(),
            score: 0.0,
            h_state: vec![0.0f32; DECODER_NUM_LAYERS * DECODER_HIDDEN_DIM],
            c_state: vec![0.0f32; DECODER_NUM_LAYERS * DECODER_HIDDEN_DIM],
            last_token: BLANK_TOKEN as i32,
            current_time: 0,
        }];

        // Safety limit
        let max_iterations = valid_time * 10;
        let mut iterations = 0;

        // Main beam search loop
        while iterations < max_iterations {
            iterations += 1;

            // Check if all beams have finished (reached end of encoder)
            let active_beams: Vec<_> = beams
                .iter()
                .filter(|b| b.current_time < valid_time)
                .collect();

            if active_beams.is_empty() {
                break;
            }

            let mut new_beams: Vec<BeamHypothesis> = Vec::new();

            for beam in beams.iter() {
                if beam.current_time >= valid_time {
                    // Beam finished, keep it as-is
                    new_beams.push(beam.clone());
                    continue;
                }

                let t = beam.current_time;

                // Create mutable states for this beam
                let mut states = LSTMStates {
                    h: beam.h_state.clone(),
                    c: beam.c_state.clone(),
                };

                // Run decoder+joint for this beam
                let logits = self.run_decoder_joint(
                    encoder_data,
                    encoder_time,
                    t,
                    beam.last_token,
                    &mut states,
                )?;

                // Get top-k tokens with their scores
                let top_k = self.get_top_k_tokens(&logits, beam_width, temperature, config.blank_penalty);
                let duration = self.get_best_duration(&logits, temperature);

                // Expand beam with top-k tokens
                for (token, log_prob) in top_k {
                    let mut new_beam = BeamHypothesis {
                        tokens: beam.tokens.clone(),
                        score: beam.score + log_prob,
                        h_state: beam.h_state.clone(),
                        c_state: beam.c_state.clone(),
                        last_token: beam.last_token,
                        current_time: beam.current_time,
                    };

                    if token == BLANK_TOKEN {
                        // Blank: advance time, keep states unchanged
                        new_beam.current_time += duration as usize;
                    } else {
                        // Token emitted: update states and advance time
                        new_beam.tokens.push(token);
                        new_beam.last_token = token as i32;
                        new_beam.h_state = states.h.clone();
                        new_beam.c_state = states.c.clone();
                        new_beam.current_time += duration as usize;
                    }

                    new_beams.push(new_beam);
                }
            }

            // Keep only top beam_width beams by score
            new_beams.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
            new_beams.truncate(beam_width);
            beams = new_beams;

            // Debug logging for first few iterations
            if iterations <= 3 {
                debug!(
                    "Beam search iter {}: {} beams, best score={:.4}, best tokens={}",
                    iterations,
                    beams.len(),
                    beams.first().map(|b| b.score).unwrap_or(0.0),
                    beams.first().map(|b| b.tokens.len()).unwrap_or(0)
                );
            }
        }

        if iterations >= max_iterations {
            warn!("Beam search reached max iterations limit");
        }

        // Return tokens from best beam
        let best_tokens = beams
            .into_iter()
            .max_by(|a, b| a.score.partial_cmp(&b.score).unwrap_or(std::cmp::Ordering::Equal))
            .map(|b| b.tokens)
            .unwrap_or_default();

        info!(
            "Beam search decoded {} tokens in {} iterations",
            best_tokens.len(),
            iterations
        );

        Ok(best_tokens)
    }

    /// Get top-k tokens with their log probabilities from logits
    fn get_top_k_tokens(&self, logits: &[f32], k: usize, temperature: f32, blank_penalty: f32) -> Vec<(u32, f32)> {
        let temp = if temperature > 0.0 { temperature } else { 1.0 };
        let token_logits = &logits[..VOCAB_SIZE];

        // Apply temperature scaling and blank penalty
        let mut scored: Vec<(u32, f32)> = token_logits
            .iter()
            .enumerate()
            .map(|(i, &val)| {
                let scaled = val / temp;
                let adjusted = if i == BLANK_TOKEN as usize {
                    scaled - blank_penalty
                } else {
                    scaled
                };
                (i as u32, adjusted)
            })
            .collect();

        // Sort by score descending
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Return top-k
        scored.truncate(k);
        scored
    }

    /// Get best duration from logits
    fn get_best_duration(&self, logits: &[f32], temperature: f32) -> u32 {
        let temp = if temperature > 0.0 { temperature } else { 1.0 };
        let duration_logits = &logits[VOCAB_SIZE..VOCAB_SIZE + NUM_DURATION_CLASSES];

        let mut max_dur = 0u32;
        let mut max_dur_val = duration_logits[0] / temp;
        for (i, &val) in duration_logits.iter().enumerate() {
            let scaled_val = val / temp;
            if scaled_val > max_dur_val {
                max_dur_val = scaled_val;
                max_dur = i as u32;
            }
        }
        max_dur + 1 // Duration is 1-indexed
    }
}
