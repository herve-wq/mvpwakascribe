use crate::engine::decoder::{TDTDecoder, Vocabulary};
use crate::engine::mel::{compute_mel_spectrogram, normalize_mel, MelConfig};
use crate::error::{AppError, Result};
use crate::storage::{Segment, Transcription};
use ndarray::Array2;
use openvino::{CompiledModel, Core};
use std::path::Path;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Fixed encoder input size (from model analysis)
const ENCODER_TIME_FRAMES: usize = 1501;
/// Encoder output dimension
const ENCODER_OUTPUT_DIM: usize = 1024;
/// Decoder hidden dimension
const DECODER_HIDDEN_DIM: usize = 640;
/// Joint output size (1030 classes: 1029 vocab tokens + 1 blank)
const JOINT_OUTPUT_SIZE: usize = 1030;
/// Blank token ID (last position in joint output)
const BLANK_ID: usize = 1029;
/// Maximum decoding steps
const MAX_DECODE_STEPS: usize = 500;

/// Parakeet STT Engine using OpenVINO with 4 separate models
pub struct ParakeetEngine {
    mel_model: Option<CompiledModel>,
    encoder: Option<CompiledModel>,
    decoder: Option<CompiledModel>,
    joint: Option<CompiledModel>,
    tdt_decoder: Option<TDTDecoder>,
    mel_config: MelConfig,
}

impl ParakeetEngine {
    pub fn new() -> Self {
        Self {
            mel_model: None,
            encoder: None,
            decoder: None,
            joint: None,
            tdt_decoder: None,
            mel_config: MelConfig::default(),
        }
    }

    /// Load the OpenVINO IR models from the model directory
    pub fn load_model(&mut self, model_dir: &Path) -> Result<()> {
        info!("Loading Parakeet models from {:?}", model_dir);

        // Initialize OpenVINO Core
        let mut core = Core::new().map_err(|e| {
            AppError::Transcription(format!("Failed to initialize OpenVINO: {}", e))
        })?;

        // Load vocabulary from JSON
        let vocab_path = model_dir.join("parakeet_vocab.json");
        if vocab_path.exists() {
            let vocab = Vocabulary::load_json(&vocab_path)?;
            info!("Loaded vocabulary with {} tokens", vocab.vocab_size());
            self.tdt_decoder = Some(TDTDecoder::new(vocab));
        } else {
            warn!("Vocabulary file not found at {:?}", vocab_path);
            return Err(AppError::Transcription(
                "Vocabulary file not found".to_string(),
            ));
        }

        // Load mel spectrogram model
        let mel_path = model_dir.join("parakeet_melspectogram.xml");
        if mel_path.exists() {
            info!("Loading mel spectrogram model...");
            match self.load_compiled_model(&mut core, &mel_path) {
                Ok(model) => {
                    self.mel_model = Some(model);
                    info!("Mel spectrogram model loaded");
                }
                Err(e) => warn!("Failed to load mel model: {}", e),
            }
        }

        // Load encoder model
        let encoder_path = model_dir.join("parakeet_encoder.xml");
        if encoder_path.exists() {
            info!("Loading encoder model...");
            match self.load_compiled_model(&mut core, &encoder_path) {
                Ok(model) => {
                    self.encoder = Some(model);
                    info!("Encoder model loaded");
                }
                Err(e) => {
                    warn!("Failed to load encoder: {}", e);
                    return Err(AppError::Transcription(format!(
                        "Failed to load encoder: {}",
                        e
                    )));
                }
            }
        } else {
            return Err(AppError::Transcription(
                "Encoder model not found".to_string(),
            ));
        }

        // Load decoder model
        let decoder_path = model_dir.join("parakeet_decoder.xml");
        if decoder_path.exists() {
            info!("Loading decoder model...");
            match self.load_compiled_model(&mut core, &decoder_path) {
                Ok(model) => {
                    self.decoder = Some(model);
                    info!("Decoder model loaded");
                }
                Err(e) => {
                    warn!("Failed to load decoder: {}", e);
                    return Err(AppError::Transcription(format!(
                        "Failed to load decoder: {}",
                        e
                    )));
                }
            }
        } else {
            return Err(AppError::Transcription(
                "Decoder model not found".to_string(),
            ));
        }

        // Load joint model
        let joint_path = model_dir.join("parakeet_joint.xml");
        if joint_path.exists() {
            info!("Loading joint model...");
            match self.load_compiled_model(&mut core, &joint_path) {
                Ok(model) => {
                    self.joint = Some(model);
                    info!("Joint model loaded");
                }
                Err(e) => {
                    warn!("Failed to load joint: {}", e);
                    return Err(AppError::Transcription(format!(
                        "Failed to load joint: {}",
                        e
                    )));
                }
            }
        } else {
            return Err(AppError::Transcription("Joint model not found".to_string()));
        }

        info!("All models loaded successfully");
        Ok(())
    }

    fn load_compiled_model(&self, core: &mut Core, path: &Path) -> Result<CompiledModel> {
        let model = core
            .read_model_from_file(path.to_str().unwrap(), "")
            .map_err(|e| AppError::Transcription(format!("Failed to read model: {}", e)))?;

        core.compile_model(&model, "CPU".into())
            .map_err(|e| AppError::Transcription(format!("Failed to compile model: {}", e)))
    }

    /// Check if all required models are loaded
    pub fn is_loaded(&self) -> bool {
        self.encoder.is_some()
            && self.decoder.is_some()
            && self.joint.is_some()
            && self.tdt_decoder.is_some()
    }

    /// Transcribe audio samples (16kHz mono f32)
    pub fn transcribe(
        &mut self,
        samples: &[f32],
        source_type: &str,
        source_name: Option<String>,
    ) -> Result<Transcription> {
        let duration_ms = (samples.len() as f64 / 16000.0 * 1000.0) as i64;

        if !self.is_loaded() {
            info!("Model not loaded, returning mock transcription");
            return self.mock_transcribe(samples, source_type, source_name);
        }

        info!(
            "Transcribing {} samples ({} ms)",
            samples.len(),
            duration_ms
        );

        // Compute mel spectrogram using our Rust implementation
        let mel_spec = compute_mel_spectrogram(samples, &self.mel_config);
        let mel_spec = normalize_mel(&mel_spec);

        debug!(
            "Mel spectrogram shape: {} x {}",
            mel_spec.nrows(),
            mel_spec.ncols()
        );

        // Run inference
        match self.run_inference(&mel_spec, duration_ms) {
            Ok((segments, raw_text)) => {
                let now = chrono::Utc::now().to_rfc3339();
                Ok(Transcription {
                    id: Uuid::new_v4().to_string(),
                    created_at: now.clone(),
                    updated_at: now,
                    source_type: source_type.to_string(),
                    source_name,
                    duration_ms,
                    language: "en".to_string(),
                    segments,
                    raw_text,
                    edited_text: None,
                    is_edited: false,
                })
            }
            Err(e) => {
                warn!(
                    "Inference failed: {}. Falling back to mock transcription.",
                    e
                );
                self.mock_transcribe(samples, source_type, source_name)
            }
        }
    }

    /// Run the full inference pipeline
    fn run_inference(
        &mut self,
        mel_spec: &Array2<f32>,
        duration_ms: i64,
    ) -> Result<(Vec<Segment>, String)> {
        // Run encoder
        let encoder_output = self.run_encoder(mel_spec)?;
        debug!("Encoder output length: {}", encoder_output.len());

        // Run greedy TDT decoding
        let token_ids = self.greedy_decode(&encoder_output)?;
        debug!("Decoded {} tokens", token_ids.len());

        // Convert tokens to text
        let decoder = self.tdt_decoder.as_ref().unwrap();
        let segments = decoder.greedy_decode(&token_ids, duration_ms);

        // Build raw text
        let raw_text: String = segments
            .iter()
            .map(|s| s.text.clone())
            .collect::<Vec<_>>()
            .join(" ");

        Ok((segments, raw_text))
    }

    /// Run encoder on mel spectrogram
    /// Input: melspectogram [1, 128, time] and melspectogram_length [1]
    /// Output: encoder_output [1, 1024, enc_time] and encoder_output_length [1]
    fn run_encoder(&mut self, mel_spec: &Array2<f32>) -> Result<Vec<f32>> {
        let encoder = self.encoder.as_mut().ok_or_else(|| {
            AppError::Transcription("Encoder not loaded".into())
        })?;

        let (n_mels, n_frames) = mel_spec.dim();
        debug!("Input mel shape: {} mels x {} frames", n_mels, n_frames);

        // Encoder expects fixed size [1, 128, 1501]
        let target_frames = ENCODER_TIME_FRAMES;
        let actual_frames = n_frames.min(target_frames);

        // Prepare input data [1, 128, 1501] - padded with zeros
        let mut input_data = vec![0.0f32; 128 * target_frames];
        for mel_idx in 0..n_mels.min(128) {
            for frame_idx in 0..actual_frames {
                input_data[mel_idx * target_frames + frame_idx] = mel_spec[[mel_idx, frame_idx]];
            }
        }

        // Create inference request
        let mut request = encoder.create_infer_request().map_err(|e| {
            AppError::Transcription(format!("Failed to create infer request: {}", e))
        })?;

        // Get and fill mel tensor by name
        {
            let mut mel_tensor = request.get_tensor("melspectogram").map_err(|e| {
                AppError::Transcription(format!("Failed to get mel tensor by name: {}", e))
            })?;

            let tensor_size = mel_tensor.get_byte_size().unwrap_or(0) / 4; // f32 = 4 bytes
            debug!("Mel tensor size: {} elements", tensor_size);

            let tensor_data = mel_tensor.get_data_mut::<f32>().map_err(|e| {
                AppError::Transcription(format!("Failed to get mel tensor data: {}", e))
            })?;

            let copy_len = tensor_data.len().min(input_data.len());
            debug!("Copying {} elements to mel tensor (tensor size: {})", copy_len, tensor_data.len());
            tensor_data[..copy_len].copy_from_slice(&input_data[..copy_len]);
        }

        // Get and fill length tensor by name
        {
            let mut length_tensor = request.get_tensor("melspectogram_length").map_err(|e| {
                AppError::Transcription(format!("Failed to get length tensor by name: {}", e))
            })?;

            let tensor_size = length_tensor.get_byte_size().unwrap_or(0) / 4; // i32 = 4 bytes
            debug!("Length tensor size: {} elements", tensor_size);

            let length_data = length_tensor.get_data_mut::<i32>().map_err(|e| {
                AppError::Transcription(format!("Failed to get length tensor data: {}", e))
            })?;
            length_data[0] = actual_frames as i32;
        }

        // Run inference
        info!("Running encoder inference...");
        request
            .infer()
            .map_err(|e| AppError::Transcription(format!("Encoder inference failed: {}", e)))?;

        // Get output by name
        let output = request.get_tensor("encoder_output").map_err(|e| {
            AppError::Transcription(format!("Failed to get encoder output: {}", e))
        })?;
        let output_data = output
            .get_data::<f32>()
            .map_err(|e| AppError::Transcription(format!("Failed to get output data: {}", e)))?;

        info!("Encoder output size: {} elements", output_data.len());
        Ok(output_data.to_vec())
    }

    /// Greedy TDT decoding using decoder and joint models
    /// Standard RNN-T greedy search: at each encoder frame, keep decoding until blank
    fn greedy_decode(&mut self, encoder_output: &[f32]) -> Result<Vec<i64>> {
        if self.decoder.is_none() {
            return Err(AppError::Transcription("Decoder not loaded".into()));
        }
        if self.joint.is_none() {
            return Err(AppError::Transcription("Joint not loaded".into()));
        }

        // Calculate number of encoder time steps
        // Encoder output is [1, 1024, T] where T = 188 for 1501 input frames
        let enc_time_steps = encoder_output.len() / ENCODER_OUTPUT_DIM;
        info!("Encoder time steps: {}", enc_time_steps);

        let mut tokens: Vec<i64> = Vec::new();

        // Initialize LSTM hidden states [2, 1, 640] with zeros
        let mut h_state = vec![0.0f32; 2 * DECODER_HIDDEN_DIM];
        let mut c_state = vec![0.0f32; 2 * DECODER_HIDDEN_DIM];

        // Initial target token - use 0 (start/unk token) for decoder
        // Note: blank is only used in joint output, not as decoder input
        let mut current_target: i32 = 0;

        // Blank token ID is defined as constant

        // RNN-T greedy decoding: for each encoder frame, decode until blank
        for t in 0..enc_time_steps {
            // Get encoder output for this time step
            let enc_start = t * ENCODER_OUTPUT_DIM;
            let enc_slice = &encoder_output[enc_start..enc_start + ENCODER_OUTPUT_DIM];

            // Inner loop: keep predicting tokens until we get a blank
            let mut max_symbols_per_step = 10; // Limit to prevent infinite loops

            loop {
                // Run decoder with current target
                let decoder_output = self.run_decoder_step(
                    current_target,
                    &h_state,
                    &c_state,
                )?;

                // Run joint to get logits
                let logits = self.run_joint(enc_slice, &decoder_output.output)?;

                // Greedy: take argmax
                let token_id = logits
                    .iter()
                    .enumerate()
                    .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
                    .map(|(idx, _)| idx)
                    .unwrap_or(BLANK_ID);

                // Debug: log first few predictions
                if tokens.len() < 5 {
                    // Find top 3 logits
                    let mut indexed: Vec<_> = logits.iter().enumerate().collect();
                    indexed.sort_by(|(_, a), (_, b)| b.partial_cmp(a).unwrap());
                    debug!(
                        "Frame {}: top logits: [{}: {:.3}], [{}: {:.3}], [{}: {:.3}], blank[{}]: {:.3}",
                        t,
                        indexed[0].0, indexed[0].1,
                        indexed[1].0, indexed[1].1,
                        indexed[2].0, indexed[2].1,
                        BLANK_ID, logits.get(BLANK_ID).unwrap_or(&0.0)
                    );
                }

                // If blank, move to next encoder frame
                if token_id == BLANK_ID {
                    break;
                }

                // Emit non-blank token
                tokens.push(token_id as i64);
                current_target = token_id as i32;

                // Update hidden states only when emitting a token
                h_state = decoder_output.h_out;
                c_state = decoder_output.c_out;

                max_symbols_per_step -= 1;
                if max_symbols_per_step == 0 {
                    debug!("Max symbols per step reached at frame {}", t);
                    break;
                }

                if tokens.len() >= MAX_DECODE_STEPS {
                    break;
                }
            }

            if tokens.len() >= MAX_DECODE_STEPS {
                break;
            }
        }

        debug!("Decoded {} tokens", tokens.len());
        Ok(tokens)
    }

    /// Run a single decoder step
    fn run_decoder_step(
        &mut self,
        target: i32,
        h_in: &[f32],
        c_in: &[f32],
    ) -> Result<DecoderOutput> {
        let decoder = self.decoder.as_mut().unwrap();

        // Create inference request
        let mut request = decoder.create_infer_request().map_err(|e| {
            AppError::Transcription(format!("Failed to create decoder request: {}", e))
        })?;

        // Fill input tensors by name
        {
            let mut c_tensor = request.get_tensor("c_in").map_err(|e| {
                AppError::Transcription(format!("Failed to get c_in tensor: {}", e))
            })?;
            let data = c_tensor.get_data_mut::<f32>().map_err(|e| {
                AppError::Transcription(format!("Failed to get c data: {}", e))
            })?;
            let copy_len = data.len().min(c_in.len());
            data[..copy_len].copy_from_slice(&c_in[..copy_len]);
        }

        {
            let mut h_tensor = request.get_tensor("h_in").map_err(|e| {
                AppError::Transcription(format!("Failed to get h_in tensor: {}", e))
            })?;
            let data = h_tensor.get_data_mut::<f32>().map_err(|e| {
                AppError::Transcription(format!("Failed to get h data: {}", e))
            })?;
            let copy_len = data.len().min(h_in.len());
            data[..copy_len].copy_from_slice(&h_in[..copy_len]);
        }

        {
            let mut target_tensor = request.get_tensor("targets").map_err(|e| {
                AppError::Transcription(format!("Failed to get targets tensor: {}", e))
            })?;
            let data = target_tensor.get_data_mut::<i32>().map_err(|e| {
                AppError::Transcription(format!("Failed to get target data: {}", e))
            })?;
            data[0] = target;
        }

        // Run inference
        request
            .infer()
            .map_err(|e| AppError::Transcription(format!("Decoder inference failed: {}", e)))?;

        // Get outputs by name
        let output_tensor = request.get_tensor("decoder_output").map_err(|e| {
            AppError::Transcription(format!("Failed to get decoder output: {}", e))
        })?;
        let output = output_tensor
            .get_data::<f32>()
            .map_err(|e| AppError::Transcription(format!("Failed to get output data: {}", e)))?
            .to_vec();

        let h_out_tensor = request.get_tensor("h_out").map_err(|e| {
            AppError::Transcription(format!("Failed to get h_out: {}", e))
        })?;
        let h_out = h_out_tensor
            .get_data::<f32>()
            .map_err(|e| AppError::Transcription(format!("Failed to get h_out data: {}", e)))?
            .to_vec();

        let c_out_tensor = request.get_tensor("c_out").map_err(|e| {
            AppError::Transcription(format!("Failed to get c_out: {}", e))
        })?;
        let c_out = c_out_tensor
            .get_data::<f32>()
            .map_err(|e| AppError::Transcription(format!("Failed to get c_out data: {}", e)))?
            .to_vec();

        Ok(DecoderOutput { output, h_out, c_out })
    }

    /// Run joint model
    fn run_joint(
        &mut self,
        encoder_output: &[f32],
        decoder_output: &[f32],
    ) -> Result<Vec<f32>> {
        let joint = self.joint.as_mut().unwrap();

        // Create inference request
        let mut request = joint.create_infer_request().map_err(|e| {
            AppError::Transcription(format!("Failed to create joint request: {}", e))
        })?;

        // Fill input tensors by name
        {
            let mut dec_tensor = request.get_tensor("decoder_outputs").map_err(|e| {
                AppError::Transcription(format!("Failed to get decoder_outputs tensor: {}", e))
            })?;
            let data = dec_tensor.get_data_mut::<f32>().map_err(|e| {
                AppError::Transcription(format!("Failed to get dec data: {}", e))
            })?;
            let copy_len = data.len().min(decoder_output.len());
            data[..copy_len].copy_from_slice(&decoder_output[..copy_len]);
        }

        {
            let mut enc_tensor = request.get_tensor("encoder_outputs").map_err(|e| {
                AppError::Transcription(format!("Failed to get encoder_outputs tensor: {}", e))
            })?;
            let data = enc_tensor.get_data_mut::<f32>().map_err(|e| {
                AppError::Transcription(format!("Failed to get enc data: {}", e))
            })?;
            let copy_len = data.len().min(encoder_output.len());
            data[..copy_len].copy_from_slice(&encoder_output[..copy_len]);
        }

        // Run inference
        request
            .infer()
            .map_err(|e| AppError::Transcription(format!("Joint inference failed: {}", e)))?;

        // Get logits output by name
        let logits_tensor = request.get_tensor("logits").map_err(|e| {
            AppError::Transcription(format!("Failed to get logits: {}", e))
        })?;
        let logits = logits_tensor
            .get_data::<f32>()
            .map_err(|e| AppError::Transcription(format!("Failed to get logits data: {}", e)))?
            .to_vec();

        Ok(logits)
    }

    /// Mock transcription for development without the model
    fn mock_transcribe(
        &self,
        samples: &[f32],
        source_type: &str,
        source_name: Option<String>,
    ) -> Result<Transcription> {
        let now = chrono::Utc::now().to_rfc3339();
        let duration_ms = (samples.len() as f64 / 16000.0 * 1000.0) as i64;

        let mock_text = "Ceci est une transcription de demonstration. \
            Le modele Parakeet n'est pas encore charge. \
            Une fois le modele OpenVINO integre, vous verrez la vraie transcription ici.";

        let segments = vec![
            Segment {
                id: Uuid::new_v4().to_string(),
                start_ms: 0,
                end_ms: duration_ms / 2,
                text: "Ceci est une transcription de demonstration.".to_string(),
                confidence: 0.85,
            },
            Segment {
                id: Uuid::new_v4().to_string(),
                start_ms: duration_ms / 2,
                end_ms: duration_ms,
                text: "Le modele Parakeet n'est pas encore charge.".to_string(),
                confidence: 0.90,
            },
        ];

        Ok(Transcription {
            id: Uuid::new_v4().to_string(),
            created_at: now.clone(),
            updated_at: now,
            source_type: source_type.to_string(),
            source_name,
            duration_ms,
            language: "fr".to_string(),
            segments,
            raw_text: mock_text.to_string(),
            edited_text: None,
            is_edited: false,
        })
    }
}

impl Default for ParakeetEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Decoder step output
struct DecoderOutput {
    output: Vec<f32>,
    h_out: Vec<f32>,
    c_out: Vec<f32>,
}
