use crate::error::{AppError, Result};
use crate::storage::Segment;
use std::collections::HashMap;
use std::path::Path;
use uuid::Uuid;

/// Vocabulary for token decoding
pub struct Vocabulary {
    tokens: Vec<String>,
    token_to_id: HashMap<String, usize>,
    // Special token IDs
    pub blank_id: usize,
    pub unk_id: usize,
}

impl Vocabulary {
    /// Load vocabulary from JSON format (parakeet_v3_vocab.json)
    /// Format: {"token_id": "token_text", ...}
    pub fn load_json(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| AppError::Io(e))?;

        let json: HashMap<String, String> = serde_json::from_str(&content)
            .map_err(|e| AppError::Transcription(format!("Failed to parse vocab JSON: {}", e)))?;

        // Find max token ID to size the vector
        let max_id = json.keys()
            .filter_map(|k| k.parse::<usize>().ok())
            .max()
            .unwrap_or(0);

        let mut tokens = vec![String::new(); max_id + 1];
        let mut token_to_id = HashMap::new();

        for (id_str, token) in json {
            if let Ok(id) = id_str.parse::<usize>() {
                tokens[id] = token.clone();
                token_to_id.insert(token, id);
            }
        }

        // Find special token IDs
        let unk_id = *token_to_id.get("<unk>").unwrap_or(&0);
        // Blank token for Parakeet TDT v3 is always 8192
        let blank_id = 8192;

        Ok(Self {
            tokens,
            token_to_id,
            blank_id,
            unk_id,
        })
    }

    /// Load vocabulary from TXT format (vocab.txt)
    /// Format: "token index\n" (e.g., "<unk> 0\n<blk> 8192\n")
    pub fn load_txt(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| AppError::Io(e))?;

        let mut tokens = Vec::new();
        let mut token_to_id = HashMap::new();
        let mut max_id = 0usize;

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            // Split from the right to handle tokens with spaces
            // Format: "token index" where index is the last element
            if let Some(last_space) = line.rfind(' ') {
                let token = &line[..last_space];
                let id_str = &line[last_space + 1..];

                if let Ok(id) = id_str.parse::<usize>() {
                    // Extend tokens vector if needed
                    if id >= tokens.len() {
                        tokens.resize(id + 1, String::new());
                    }
                    tokens[id] = token.to_string();
                    token_to_id.insert(token.to_string(), id);
                    max_id = max_id.max(id);
                }
            }
        }

        // Find special token IDs
        let unk_id = *token_to_id.get("<unk>").unwrap_or(&0);
        // Blank token is "<blk>" in TXT format, always 8192
        let blank_id = *token_to_id.get("<blk>").unwrap_or(&8192);

        Ok(Self {
            tokens,
            token_to_id,
            blank_id,
            unk_id,
        })
    }

    /// Load vocabulary, auto-detecting format from extension
    pub fn load(path: &Path) -> Result<Self> {
        match path.extension().and_then(|e| e.to_str()) {
            Some("json") => Self::load_json(path),
            Some("txt") => Self::load_txt(path),
            _ => Err(AppError::Transcription(format!(
                "Unknown vocabulary format: {:?}", path
            ))),
        }
    }

    pub fn decode_token(&self, id: usize) -> &str {
        self.tokens.get(id).map(|s| s.as_str()).unwrap_or("<unk>")
    }

    pub fn vocab_size(&self) -> usize {
        self.tokens.len()
    }

    pub fn is_special_token(&self, id: usize) -> bool {
        // Only the blank token (8192) is considered special for TDT decoding
        id == self.blank_id
    }
}

/// TDT Decoder for converting model outputs to text
pub struct TDTDecoder {
    vocab: Vocabulary,
}

impl TDTDecoder {
    pub fn new(vocab: Vocabulary) -> Self {
        Self { vocab }
    }

    pub fn vocab(&self) -> &Vocabulary {
        &self.vocab
    }

    /// Decode token IDs to text segments with timestamps
    pub fn decode(
        &self,
        token_ids: &[i64],
        durations: &[i64],
        sample_rate: u32,
        hop_length: usize,
        subsampling_factor: usize,
    ) -> Vec<Segment> {
        let mut segments = Vec::new();
        let mut current_text = String::new();
        let segment_start_ms: i64 = 0;
        let mut current_frame: i64 = 0;
        let mut confidence_sum = 0.0;
        let mut token_count = 0;

        // Time per frame in milliseconds
        let ms_per_frame = (hop_length * subsampling_factor) as f64 / sample_rate as f64 * 1000.0;

        for (i, &token_id) in token_ids.iter().enumerate() {
            let token_id = token_id as usize;
            let duration = durations.get(i).copied().unwrap_or(1) as i64;

            // Skip special tokens
            if self.vocab.is_special_token(token_id) {
                current_frame += duration;
                continue;
            }

            // Decode token
            let token_text = self.vocab.decode_token(token_id);

            // Handle SentencePiece tokens (▁ prefix means word boundary)
            if token_text.starts_with("▁") {
                current_text.push_str(&token_text.replace("▁", " "));
            } else {
                current_text.push_str(token_text);
            }

            confidence_sum += 0.9;
            token_count += 1;
            current_frame += duration;
        }

        // Final segment
        if !current_text.is_empty() {
            let end_ms = (current_frame as f64 * ms_per_frame) as i64;
            segments.push(Segment {
                id: Uuid::new_v4().to_string(),
                start_ms: segment_start_ms,
                end_ms,
                text: current_text.trim().to_string(),
                confidence: if token_count > 0 {
                    confidence_sum / token_count as f64
                } else {
                    0.9
                },
            });
        }

        segments
    }

    /// Simple greedy decode without durations
    pub fn greedy_decode(&self, token_ids: &[i64], duration_ms: i64) -> Vec<Segment> {
        let mut text = String::new();

        for &token_id in token_ids {
            let token_id = token_id as usize;

            if self.vocab.is_special_token(token_id) {
                continue;
            }

            let token_text = self.vocab.decode_token(token_id);

            // Handle SentencePiece tokens
            if token_text.starts_with("▁") {
                text.push_str(&token_text.replace("▁", " "));
            } else {
                text.push_str(token_text);
            }
        }

        let text = text.trim().to_string();
        if text.is_empty() {
            return vec![];
        }

        vec![Segment {
            id: Uuid::new_v4().to_string(),
            start_ms: 0,
            end_ms: duration_ms,
            text,
            confidence: 0.9,
        }]
    }

    /// Decode a single token ID to text
    pub fn decode_single(&self, token_id: usize) -> String {
        // Skip blank token (8192) and out-of-range tokens
        if token_id == 8192 || token_id >= self.vocab.vocab_size() {
            return String::new();
        }
        let token = self.vocab.decode_token(token_id);
        token.replace("▁", " ")
    }
}
