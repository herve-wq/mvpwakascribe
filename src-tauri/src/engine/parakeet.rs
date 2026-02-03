use crate::audio::{split_audio_smart, SmartChunkConfig};
use crate::engine::decoder::{TDTDecoder, Vocabulary};
use crate::error::{AppError, Result};
use crate::storage::{Segment, Transcription};
use openvino::{CompiledModel, Core, DeviceType, InferRequest};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Mutex;
use tracing::{debug, info, warn};
use uuid::Uuid;

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
}

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

/// Maximum audio samples (15 seconds at 16kHz)
const MAX_AUDIO_SAMPLES: usize = 240000;

/// Maximum mel frames
const MAX_MEL_FRAMES: usize = 1501;

/// Mel features dimension
const MEL_FEATURES: usize = 128;

/// Encoder output dimension
const ENCODER_OUTPUT_DIM: usize = 1024;

/// Encoder output time dimension (fixed tensor size, even if valid frames < this)
const MAX_ENCODER_TIME: usize = 188;

/// Hop length for mel spectrogram (samples per frame)
const HOP_LENGTH: usize = 160;

/// Blank penalty: valeur à soustraire du logit du blank token
/// Augmenter cette valeur réduit le biais vers blank
const BLANK_PENALTY: f32 = 6.0;

/// Parakeet STT Engine using OpenVINO with 4 separate models
pub struct ParakeetEngine {
    #[allow(dead_code)]
    core: Option<Mutex<Core>>,
    mel_request: Option<Mutex<InferRequest>>,
    encoder_request: Option<Mutex<InferRequest>>,
    decoder_request: Option<Mutex<InferRequest>>,
    joint_request: Option<Mutex<InferRequest>>,
    tdt_decoder: Option<TDTDecoder>,
    // Store compiled models to recreate InferRequests between transcriptions
    mel_model: Option<Mutex<CompiledModel>>,
    encoder_model: Option<Mutex<CompiledModel>>,
    decoder_model: Option<Mutex<CompiledModel>>,
    joint_model: Option<Mutex<CompiledModel>>,
}

// Implement Send + Sync manually since InferRequest might not be Sync
unsafe impl Send for ParakeetEngine {}
unsafe impl Sync for ParakeetEngine {}

impl ParakeetEngine {
    pub fn new() -> Self {
        Self {
            core: None,
            mel_request: None,
            encoder_request: None,
            decoder_request: None,
            joint_request: None,
            tdt_decoder: None,
            mel_model: None,
            encoder_model: None,
            decoder_model: None,
            joint_model: None,
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
        let vocab_path = model_dir.join("parakeet_v3_vocab.json");
        let vocab_path = if vocab_path.exists() {
            vocab_path
        } else {
            model_dir.join("parakeet_vocab.json")
        };

        if vocab_path.exists() {
            let vocab = Vocabulary::load_json(&vocab_path)?;
            info!("Loaded vocabulary with {} tokens from {:?}", vocab.vocab_size(), vocab_path);
            self.tdt_decoder = Some(TDTDecoder::new(vocab));
        } else {
            return Err(AppError::Transcription("Vocabulary file not found".to_string()));
        }

        // Load mel spectrogram model
        info!("Loading mel spectrogram model...");
        let mut mel_model = Self::load_compiled_model(&mut core, model_dir, "parakeet_melspectogram")?;
        let mel_request = mel_model.create_infer_request().map_err(|e| {
            AppError::Transcription(format!("Failed to create mel infer request: {}", e))
        })?;
        info!("Mel spectrogram model loaded");

        // Load encoder model
        info!("Loading encoder model...");
        let mut encoder_model = Self::load_compiled_model(&mut core, model_dir, "parakeet_encoder")?;
        let encoder_request = encoder_model.create_infer_request().map_err(|e| {
            AppError::Transcription(format!("Failed to create encoder infer request: {}", e))
        })?;
        info!("Encoder model loaded");

        // Load decoder model
        info!("Loading decoder model...");
        let mut decoder_model = Self::load_compiled_model(&mut core, model_dir, "parakeet_decoder")?;
        let decoder_request = decoder_model.create_infer_request().map_err(|e| {
            AppError::Transcription(format!("Failed to create decoder infer request: {}", e))
        })?;
        info!("Decoder model loaded");

        // Load joint model
        info!("Loading joint model...");
        let mut joint_model = Self::load_compiled_model(&mut core, model_dir, "parakeet_joint")?;
        let joint_request = joint_model.create_infer_request().map_err(|e| {
            AppError::Transcription(format!("Failed to create joint infer request: {}", e))
        })?;
        info!("Joint model loaded");

        // Store everything
        self.core = Some(Mutex::new(core));
        self.mel_request = Some(Mutex::new(mel_request));
        self.encoder_request = Some(Mutex::new(encoder_request));
        self.decoder_request = Some(Mutex::new(decoder_request));
        self.joint_request = Some(Mutex::new(joint_request));
        self.mel_model = Some(Mutex::new(mel_model));
        self.encoder_model = Some(Mutex::new(encoder_model));
        self.decoder_model = Some(Mutex::new(decoder_model));
        self.joint_model = Some(Mutex::new(joint_model));

        info!("All models loaded successfully");
        Ok(())
    }

    /// Recreate all InferRequests to ensure clean state between transcriptions
    fn reset_all_requests(&self) -> Result<()> {
        // Reset mel request
        if let (Some(model), Some(request)) = (&self.mel_model, &self.mel_request) {
            let mut model = model.lock().unwrap();
            let new_request = model.create_infer_request().map_err(|e| {
                AppError::Transcription(format!("Failed to recreate mel infer request: {}", e))
            })?;
            let mut request = request.lock().unwrap();
            *request = new_request;
        }

        // Reset encoder request
        if let (Some(model), Some(request)) = (&self.encoder_model, &self.encoder_request) {
            let mut model = model.lock().unwrap();
            let new_request = model.create_infer_request().map_err(|e| {
                AppError::Transcription(format!("Failed to recreate encoder infer request: {}", e))
            })?;
            let mut request = request.lock().unwrap();
            *request = new_request;
        }

        // Reset decoder request
        if let (Some(model), Some(request)) = (&self.decoder_model, &self.decoder_request) {
            let mut model = model.lock().unwrap();
            let new_request = model.create_infer_request().map_err(|e| {
                AppError::Transcription(format!("Failed to recreate decoder infer request: {}", e))
            })?;
            let mut request = request.lock().unwrap();
            *request = new_request;
        }

        // Reset joint request
        if let (Some(model), Some(request)) = (&self.joint_model, &self.joint_request) {
            let mut model = model.lock().unwrap();
            let new_request = model.create_infer_request().map_err(|e| {
                AppError::Transcription(format!("Failed to recreate joint infer request: {}", e))
            })?;
            let mut request = request.lock().unwrap();
            *request = new_request;
        }

        debug!("All InferRequests recreated to clear internal state");
        Ok(())
    }

    fn load_compiled_model(core: &mut Core, model_dir: &Path, model_name: &str) -> Result<CompiledModel> {
        let xml_path = model_dir.join(format!("{}.xml", model_name));
        let bin_path = model_dir.join(format!("{}.bin", model_name));

        if !xml_path.exists() {
            return Err(AppError::Transcription(format!("XML file not found: {:?}", xml_path)));
        }

        // Load model - bin file is optional (weights may be embedded in XML)
        let bin_path_str = if bin_path.exists() {
            bin_path.to_str().unwrap()
        } else {
            info!("No .bin file for {}, using embedded weights", model_name);
            ""
        };

        let model = core.read_model_from_file(
            xml_path.to_str().unwrap(),
            bin_path_str,
        ).map_err(|e| AppError::Transcription(format!("Failed to read model {}: {}", model_name, e)))?;

        core.compile_model(&model, DeviceType::CPU)
            .map_err(|e| AppError::Transcription(format!("Failed to compile model {}: {}", model_name, e)))
    }

    /// Check if all required models are loaded
    pub fn is_loaded(&self) -> bool {
        self.mel_request.is_some()
            && self.encoder_request.is_some()
            && self.decoder_request.is_some()
            && self.joint_request.is_some()
            && self.tdt_decoder.is_some()
    }

    /// Transcribe audio samples (16kHz mono f32)
    pub fn transcribe(
        &self,
        samples: &[f32],
        source_type: &str,
        source_name: Option<String>,
        language: TranscriptionLanguage,
    ) -> Result<Transcription> {
        let duration_ms = (samples.len() as f64 / 16000.0 * 1000.0) as i64;

        if !self.is_loaded() {
            info!("Model not loaded, returning mock transcription");
            return self.mock_transcribe(samples, source_type, source_name);
        }

        info!(
            "Transcribing {} samples ({} ms) with language: {:?}",
            samples.len(),
            duration_ms,
            language
        );

        match self.run_inference(samples, language) {
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
                    language: "en".to_string(),
                    segments,
                    raw_text: text,
                    edited_text: None,
                    is_edited: false,
                })
            }
            Err(e) => {
                warn!("Inference failed: {}. Falling back to mock transcription.", e);
                self.mock_transcribe(samples, source_type, source_name)
            }
        }
    }

    /// Pipeline complet de transcription TDT avec support chunking
    fn run_inference(&self, audio: &[f32], language: TranscriptionLanguage) -> Result<String> {
        info!("Starting TDT inference on {} audio samples", audio.len());

        // Check if audio needs chunking
        if audio.len() > MAX_AUDIO_SAMPLES {
            info!(
                "Audio too long ({} samples = {:.1}s), using chunked transcription",
                audio.len(),
                audio.len() as f32 / 16000.0
            );
            return self.run_chunked_inference(audio, language);
        }

        // Single chunk inference
        self.run_single_inference(audio, language)
    }

    /// Run inference on a single chunk (max 15s)
    fn run_single_inference(&self, audio: &[f32], language: TranscriptionLanguage) -> Result<String> {
        // Reset all InferRequests to ensure clean state
        self.reset_all_requests()?;

        // DIAGNOSTIC: Audio stats
        let (audio_min, audio_max, audio_rms) = compute_stats(audio);
        info!(
            "DIAG Audio: min={:.4}, max={:.4}, rms={:.4}",
            audio_min, audio_max, audio_rms
        );

        // Étape 1: Calculer le Mel Spectrogram
        let mel_features = self.compute_mel_spectrogram(audio)?;
        let time_frames = mel_features.len() / MEL_FEATURES;

        // FIX: Calculer le nombre réel de frames mel valides basé sur la longueur audio
        // (le tensor mel a une taille fixe de 1501, mais seules les frames correspondant
        // à l'audio réel sont valides)
        let actual_audio_len = audio.len().min(MAX_AUDIO_SAMPLES);
        let actual_mel_frames = (actual_audio_len / HOP_LENGTH).min(MAX_MEL_FRAMES);

        // DIAGNOSTIC: Mel stats
        let (mel_min, mel_max, mel_rms) = compute_stats(&mel_features);
        let mel_nonzero = count_nonzero(&mel_features);
        info!(
            "DIAG Mel: min={:.4}, max={:.4}, rms={:.4}, non-zero={:.1}%",
            mel_min, mel_max, mel_rms, mel_nonzero * 100.0
        );
        info!(
            "Mel spectrogram: {} tensor frames, {} actual valid frames",
            time_frames, actual_mel_frames
        );

        if actual_mel_frames == 0 {
            return Err(AppError::Transcription(
                "Mel spectrogram produced 0 time frames".to_string(),
            ));
        }

        // Étape 2: Encoder (passer le nombre réel de frames, pas la taille du tensor)
        let (encoder_output, valid_encoder_time) =
            self.run_encoder(&mel_features, actual_mel_frames)?;
        let encoder_tensor_time = encoder_output.len() / ENCODER_OUTPUT_DIM;

        // DIAGNOSTIC: Encoder stats
        let (enc_min, enc_max, enc_rms) = compute_stats(&encoder_output);
        let enc_nonzero = count_nonzero(&encoder_output);
        info!(
            "DIAG Encoder: min={:.4}, max={:.4}, rms={:.4}, non-zero={:.1}%",
            enc_min, enc_max, enc_rms, enc_nonzero * 100.0
        );
        info!(
            "Encoder output: {} tensor time steps, {} valid time steps",
            encoder_tensor_time, valid_encoder_time
        );

        if valid_encoder_time == 0 {
            return Err(AppError::Transcription(
                "Encoder produced 0 valid time steps".to_string(),
            ));
        }

        // Étape 3: Décodage TDT greedy (utiliser seulement les time steps valides!)
        let tokens = self.tdt_greedy_decode(&encoder_output, valid_encoder_time, language)?;
        info!("TDT decoding produced {} tokens", tokens.len());

        // Étape 4: Convertir tokens en texte
        let decoder = self.tdt_decoder.as_ref().unwrap();
        let text: String = tokens
            .iter()
            .map(|&t| decoder.decode_single(t as usize))
            .collect::<Vec<_>>()
            .join("");
        let text = text.trim().to_string();
        info!("Decoded text: '{}'", text);

        Ok(text)
    }

    /// Run chunked inference for long audio using VAD-based smart chunking
    ///
    /// Instead of fixed overlap, this cuts at silence points to avoid
    /// splitting words. The resulting chunks can be simply concatenated.
    fn run_chunked_inference(&self, audio: &[f32], language: TranscriptionLanguage) -> Result<String> {
        // Use smart VAD-based chunking (cuts at silence points)
        let config = SmartChunkConfig::default(); // 8-14s, cuts at silence
        let chunks = split_audio_smart(audio, &config);

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

            match self.run_single_inference(&chunk.samples, language) {
                Ok(text) => {
                    let text = text.trim().to_string();
                    if !text.is_empty() {
                        info!("Chunk {} transcription: '{}'", i + 1, text);
                        transcriptions.push(text);
                    } else {
                        debug!("Chunk {} produced empty transcription (silence?)", i + 1);
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

    /// Calcule le mel spectrogram à partir de l'audio brut
    fn compute_mel_spectrogram(&self, audio: &[f32]) -> Result<Vec<f32>> {
        let mel_request = self.mel_request.as_ref().unwrap();
        let mut mel_request = mel_request.lock().unwrap();

        // Préparer l'audio: padder ou tronquer à MAX_AUDIO_SAMPLES
        let actual_len = audio.len().min(MAX_AUDIO_SAMPLES);
        let mut padded_audio = vec![0.0f32; MAX_AUDIO_SAMPLES];
        padded_audio[..actual_len].copy_from_slice(&audio[..actual_len]);

        debug!("Mel input: {} actual samples, padded to {}", actual_len, MAX_AUDIO_SAMPLES);

        // Récupérer le tensor d'entrée pré-alloué par le modèle
        let mut input_tensor = mel_request.get_tensor("input_signals")
            .map_err(|e| AppError::Transcription(format!("mel get input tensor: {:?}", e)))?;
        {
            let data = input_tensor.get_data_mut::<f32>()
                .map_err(|e| AppError::Transcription(format!("mel input data: {:?}", e)))?;
            data.fill(0.0);
            data[..MAX_AUDIO_SAMPLES].copy_from_slice(&padded_audio);
        }

        // Input length: [1]
        let mut length_tensor = mel_request.get_tensor("input_length")
            .map_err(|e| AppError::Transcription(format!("mel get length tensor: {:?}", e)))?;
        length_tensor.get_data_mut::<i64>()
            .map_err(|e| AppError::Transcription(format!("mel length data: {:?}", e)))?[0] = actual_len as i64;

        // Inférence
        info!("Running mel spectrogram model...");
        mel_request.infer()
            .map_err(|e| AppError::Transcription(format!("mel infer: {:?}", e)))?;

        // Récupérer la sortie
        let output_tensor = mel_request.get_output_tensor_by_index(0)
            .map_err(|e| AppError::Transcription(format!("mel get output: {:?}", e)))?;

        let output_data = output_tensor.get_data::<f32>()
            .map_err(|e| AppError::Transcription(format!("mel output data: {:?}", e)))?;

        info!("Mel output size: {} elements", output_data.len());

        Ok(output_data.to_vec())
    }

    /// Encode les features mel avec l'encoder FastConformer
    /// actual_valid_frames: nombre de frames mel réellement valides (basé sur la longueur audio)
    /// Returns: (encoder_output, valid_encoder_time_steps)
    fn run_encoder(&self, mel_features: &[f32], actual_valid_frames: usize) -> Result<(Vec<f32>, usize)> {
        let encoder_request = self.encoder_request.as_ref().unwrap();
        let mut encoder_request = encoder_request.lock().unwrap();

        // Le tensor mel a toujours shape [128, 1501], donc stride = MAX_MEL_FRAMES
        let mel_tensor_stride = MAX_MEL_FRAMES;
        let frames_to_copy = actual_valid_frames.min(MAX_MEL_FRAMES);
        let mut padded_mel = vec![0.0f32; MEL_FEATURES * MAX_MEL_FRAMES];

        // Copier seulement les frames valides (shape: [128, 1501] -> [128, 1501])
        for f in 0..MEL_FEATURES {
            for t in 0..frames_to_copy {
                let src_idx = f * mel_tensor_stride + t;
                if src_idx < mel_features.len() {
                    padded_mel[f * MAX_MEL_FRAMES + t] = mel_features[src_idx];
                }
            }
        }

        debug!("Encoder input: {} valid frames (of {} tensor frames)", frames_to_copy, mel_tensor_stride);

        // Récupérer le tensor d'entrée pré-alloué
        let mut input_tensor = encoder_request.get_tensor("melspectogram")
            .map_err(|e| AppError::Transcription(format!("encoder get input tensor: {:?}", e)))?;
        {
            let data = input_tensor.get_data_mut::<f32>()
                .map_err(|e| AppError::Transcription(format!("encoder input data: {:?}", e)))?;
            data.fill(0.0);
            data[..padded_mel.len()].copy_from_slice(&padded_mel);
        }

        // Input length: [1] - passer le nombre réel de frames valides
        let mut length_tensor = encoder_request.get_tensor("melspectogram_length")
            .map_err(|e| AppError::Transcription(format!("encoder get length tensor: {:?}", e)))?;
        length_tensor.get_data_mut::<i32>()
            .map_err(|e| AppError::Transcription(format!("encoder length data: {:?}", e)))?[0] = frames_to_copy as i32;

        // Inférence
        info!("Running encoder inference...");
        encoder_request.infer()
            .map_err(|e| AppError::Transcription(format!("encoder infer: {:?}", e)))?;

        // Récupérer la sortie des features
        let output_tensor = encoder_request.get_tensor("encoder_output")
            .map_err(|e| AppError::Transcription(format!("encoder get output: {:?}", e)))?;

        let output_data = output_tensor.get_data::<f32>()
            .map_err(|e| AppError::Transcription(format!("encoder output data: {:?}", e)))?;

        // FIX: Récupérer encoder_output_length pour savoir combien de time steps sont valides
        let length_output = encoder_request.get_tensor("encoder_output_length")
            .map_err(|e| AppError::Transcription(format!("encoder get output_length: {:?}", e)))?;

        let valid_time_steps = length_output.get_data::<i64>()
            .map_err(|e| AppError::Transcription(format!("encoder output_length data: {:?}", e)))?[0] as usize;

        info!("Encoder output size: {} elements, valid time steps: {}", output_data.len(), valid_time_steps);

        Ok((output_data.to_vec(), valid_time_steps))
    }

    /// Décodage TDT greedy avec le decoder LSTM et le joint network
    fn tdt_greedy_decode(
        &self,
        encoder_output: &[f32],
        encoder_time: usize,
        language: TranscriptionLanguage,
    ) -> Result<Vec<u32>> {
        let decoder_request = self.decoder_request.as_ref().unwrap();
        let joint_request = self.joint_request.as_ref().unwrap();
        let mut decoder_request = decoder_request.lock().unwrap();
        let mut joint_request = joint_request.lock().unwrap();

        // États LSTM initiaux (zeros)
        let mut h_state = vec![0.0f32; DECODER_NUM_LAYERS * DECODER_HIDDEN_DIM];
        let mut c_state = vec![0.0f32; DECODER_NUM_LAYERS * DECODER_HIDDEN_DIM];

        // Token actuel (commence avec blank ou token de langue)
        let mut last_token: i64 = BLANK_TOKEN as i64;

        let mut tokens: Vec<u32> = Vec::new();

        // Si une langue est forcée, initialiser le decoder avec le token de langue
        if let Some(lang_token) = language.token_id() {
            info!(
                "Forcing language with token {} ({})",
                lang_token,
                language.display_name()
            );

            // Exécuter une étape du décodeur avec le token de langue
            // pour conditionner l'état LSTM
            let (_, new_h, new_c) = self.run_decoder_step(
                &mut decoder_request,
                lang_token,
                &h_state,
                &c_state,
            )?;

            // Mettre à jour les états LSTM avec le contexte de langue
            h_state = new_h;
            c_state = new_c;
            last_token = lang_token;

            debug!("Decoder conditioned with language token");
        }
        let mut t: usize = 0;

        // Limite de sécurité
        let max_iterations = encoder_time * 10;
        let mut iterations = 0;

        // Buffer pour extraire une frame temporelle
        let mut encoder_frame = vec![0.0f32; ENCODER_OUTPUT_DIM];

        while t < encoder_time && iterations < max_iterations {
            iterations += 1;

            // Extraire la frame temporelle t de l'encoder output
            // Shape est [1, 1024, 188] - le tensor a toujours 188 timesteps même si seuls encoder_time sont valides
            for i in 0..ENCODER_OUTPUT_DIM {
                encoder_frame[i] = encoder_output[i * MAX_ENCODER_TIME + t];
            }

            // DIAGNOSTIC: Pour les premières itérations, logger les stats de l'encoder frame
            if iterations <= 3 {
                let (ef_min, ef_max, ef_rms) = compute_stats(&encoder_frame);
                debug!("Encoder frame t={}: min={:.4}, max={:.4}, rms={:.4}", t, ef_min, ef_max, ef_rms);
            }

            // Étape 1: Decoder
            let (dec_out, new_h, new_c) = self.run_decoder_step(
                &mut decoder_request,
                last_token,
                &h_state,
                &c_state,
            )?;

            // DIAGNOSTIC: Pour les premières itérations, logger les stats du decoder output
            if iterations <= 3 {
                let (do_min, do_max, do_rms) = compute_stats(&dec_out);
                debug!("Decoder out t={}: min={:.4}, max={:.4}, rms={:.4}, len={}", t, do_min, do_max, do_rms, dec_out.len());
            }

            // Étape 2: Joint network
            let logits = self.run_joint_step(
                &mut joint_request,
                &encoder_frame,
                &dec_out,
            )?;

            // Étape 3: Decode TDT output
            let (token, duration) = self.decode_tdt_output(&logits);

            // Debug log for first few iterations - with logits analysis
            if iterations <= 5 || tokens.len() < 10 {
                let vocab = self.tdt_decoder.as_ref().map(|d| d.vocab());
                let token_text = if token == BLANK_TOKEN {
                    "<blank>".to_string()
                } else {
                    vocab.map(|v| v.decode_token(token as usize)).unwrap_or("?").to_string()
                };

                // Analyze logits to understand why blank is chosen
                let blank_logit = logits[BLANK_TOKEN as usize];
                // Find best non-blank token
                let (best_nonblank, best_nonblank_logit) = logits[..BLANK_TOKEN as usize]
                    .iter()
                    .enumerate()
                    .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
                    .map(|(i, v)| (i, *v))
                    .unwrap_or((0, f32::NEG_INFINITY));
                let best_nonblank_text = vocab.map(|v| v.decode_token(best_nonblank)).unwrap_or("?");

                debug!("t={}, token={} ('{}'), dur={} | blank_logit={:.4}, best_nonblank={}('{}')={:.4}, margin={:.4}",
                    t, token, token_text, duration,
                    blank_logit, best_nonblank, best_nonblank_text, best_nonblank_logit,
                    blank_logit - best_nonblank_logit);
            }

            if token == BLANK_TOKEN {
                // Blank: avancer dans le temps
                t += duration as usize;
            } else {
                // Token émis
                tokens.push(token);
                last_token = token as i64;
                h_state = new_h;
                c_state = new_c;
                t += duration as usize;
            }
        }

        if iterations >= max_iterations {
            warn!("TDT decoding reached max iterations limit");
        }

        info!("Decoded {} tokens in {} iterations", tokens.len(), iterations);
        Ok(tokens)
    }

    /// Exécute une étape du decoder LSTM
    fn run_decoder_step(
        &self,
        request: &mut InferRequest,
        target: i64,
        h_in: &[f32],
        c_in: &[f32],
    ) -> Result<(Vec<f32>, Vec<f32>, Vec<f32>)> {
        // Target: [1, 1]
        let mut target_tensor = request.get_tensor("targets")
            .map_err(|e| AppError::Transcription(format!("decoder get targets tensor: {:?}", e)))?;
        target_tensor.get_data_mut::<i64>()
            .map_err(|e| AppError::Transcription(format!("decoder target data: {:?}", e)))?[0] = target;

        // H_in: [2, 1, 640]
        let mut h_tensor = request.get_tensor("h_in")
            .map_err(|e| AppError::Transcription(format!("decoder get h_in tensor: {:?}", e)))?;
        {
            let data = h_tensor.get_data_mut::<f32>()
                .map_err(|e| AppError::Transcription(format!("decoder h_in data: {:?}", e)))?;
            data.fill(0.0);
            data[..h_in.len()].copy_from_slice(h_in);
        }

        // C_in: [2, 1, 640]
        let mut c_tensor = request.get_tensor("c_in")
            .map_err(|e| AppError::Transcription(format!("decoder get c_in tensor: {:?}", e)))?;
        {
            let data = c_tensor.get_data_mut::<f32>()
                .map_err(|e| AppError::Transcription(format!("decoder c_in data: {:?}", e)))?;
            data.fill(0.0);
            data[..c_in.len()].copy_from_slice(c_in);
        }

        // Inférence
        request.infer()
            .map_err(|e| AppError::Transcription(format!("decoder infer: {:?}", e)))?;

        // Récupérer les sorties
        let dec_output = request.get_tensor("decoder_output")
            .map_err(|e| AppError::Transcription(format!("decoder get output: {:?}", e)))?;
        let h_out = request.get_tensor("h_out")
            .map_err(|e| AppError::Transcription(format!("decoder get h_out: {:?}", e)))?;
        let c_out = request.get_tensor("c_out")
            .map_err(|e| AppError::Transcription(format!("decoder get c_out: {:?}", e)))?;

        let dec_data = dec_output.get_data::<f32>()
            .map_err(|e| AppError::Transcription(format!("decoder output data: {:?}", e)))?
            .to_vec();
        let h_data = h_out.get_data::<f32>()
            .map_err(|e| AppError::Transcription(format!("decoder h_out data: {:?}", e)))?
            .to_vec();
        let c_data = c_out.get_data::<f32>()
            .map_err(|e| AppError::Transcription(format!("decoder c_out data: {:?}", e)))?
            .to_vec();

        Ok((dec_data, h_data, c_data))
    }

    /// Exécute le joint network
    fn run_joint_step(
        &self,
        request: &mut InferRequest,
        encoder_frame: &[f32],
        decoder_output: &[f32],
    ) -> Result<Vec<f32>> {
        // Encoder output: [1, 1, 1024]
        let mut enc_tensor = request.get_tensor("encoder_outputs")
            .map_err(|e| AppError::Transcription(format!("joint get encoder tensor: {:?}", e)))?;
        {
            let data = enc_tensor.get_data_mut::<f32>()
                .map_err(|e| AppError::Transcription(format!("joint enc data: {:?}", e)))?;
            data.fill(0.0);
            data[..encoder_frame.len()].copy_from_slice(encoder_frame);
        }

        // Decoder output: [1, 1, 640]
        let mut dec_tensor = request.get_tensor("decoder_outputs")
            .map_err(|e| AppError::Transcription(format!("joint get decoder tensor: {:?}", e)))?;
        {
            let data = dec_tensor.get_data_mut::<f32>()
                .map_err(|e| AppError::Transcription(format!("joint dec data: {:?}", e)))?;
            data.fill(0.0);
            data[..decoder_output.len()].copy_from_slice(decoder_output);
        }

        // Inférence
        request.infer()
            .map_err(|e| AppError::Transcription(format!("joint infer: {:?}", e)))?;

        // Récupérer les logits
        let output = request.get_output_tensor()
            .map_err(|e| AppError::Transcription(format!("joint get output: {:?}", e)))?;

        let logits = output.get_data::<f32>()
            .map_err(|e| AppError::Transcription(format!("joint output data: {:?}", e)))?
            .to_vec();

        Ok(logits)
    }

    /// Décode la sortie TDT: token + durée
    fn decode_tdt_output(&self, logits: &[f32]) -> (u32, u32) {
        // Les premiers VOCAB_SIZE logits sont pour les tokens
        let token_logits = &logits[..VOCAB_SIZE];
        let mut max_token = 0u32;
        let mut max_token_val = token_logits[0];
        for (i, &val) in token_logits.iter().enumerate() {
            // Appliquer la pénalité blank pour réduire le biais vers blank
            let adjusted_val = if i == BLANK_TOKEN as usize {
                val - BLANK_PENALTY
            } else {
                val
            };
            if adjusted_val > max_token_val {
                max_token_val = adjusted_val;
                max_token = i as u32;
            }
        }

        // Les NUM_DURATION_CLASSES derniers sont pour les durées
        let duration_logits = &logits[VOCAB_SIZE..VOCAB_SIZE + NUM_DURATION_CLASSES];
        let mut max_dur = 0u32;
        let mut max_dur_val = duration_logits[0];
        for (i, &val) in duration_logits.iter().enumerate() {
            if val > max_dur_val {
                max_dur_val = val;
                max_dur = i as u32;
            }
        }

        // La durée est 1-indexée
        let duration = max_dur + 1;

        (max_token, duration)
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
            Le modele Parakeet n'est pas encore charge.";

        let segments = vec![Segment {
            id: Uuid::new_v4().to_string(),
            start_ms: 0,
            end_ms: duration_ms,
            text: mock_text.to_string(),
            confidence: 0.85,
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

/// DIAGNOSTIC: Compute min, max, rms of a slice
fn compute_stats(data: &[f32]) -> (f32, f32, f32) {
    if data.is_empty() {
        return (0.0, 0.0, 0.0);
    }
    let mut min = f32::MAX;
    let mut max = f32::MIN;
    let mut sum_sq = 0.0f64;
    for &v in data {
        if v < min { min = v; }
        if v > max { max = v; }
        sum_sq += (v as f64) * (v as f64);
    }
    let rms = (sum_sq / data.len() as f64).sqrt() as f32;
    (min, max, rms)
}

/// DIAGNOSTIC: Count percentage of non-zero values
fn count_nonzero(data: &[f32]) -> f32 {
    if data.is_empty() {
        return 0.0;
    }
    let nonzero = data.iter().filter(|&&v| v.abs() > 1e-9).count();
    nonzero as f32 / data.len() as f32
}
