use serde::{Deserialize, Serialize};

/// Configuration for the TDT decoding process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecodingConfig {
    /// Beam width for beam search (1 = greedy, 5-10 recommended for quality)
    pub beam_width: usize,
    /// Temperature for logits scaling (0.1-1.5, lower = more conservative)
    pub temperature: f32,
    /// Blank penalty: value subtracted from blank token logit (0-15, higher = more tokens)
    pub blank_penalty: f32,
}

impl Default for DecodingConfig {
    fn default() -> Self {
        Self {
            beam_width: 1,      // Greedy decoding by default (fastest)
            temperature: 1.0,   // No scaling by default
            blank_penalty: 6.0, // Default blank penalty
        }
    }
}

impl DecodingConfig {
    /// Create a config for greedy decoding (fastest)
    pub fn greedy() -> Self {
        Self::default()
    }

    /// Create a config for beam search with recommended settings
    pub fn beam_search(beam_width: usize) -> Self {
        Self {
            beam_width: beam_width.max(1),
            temperature: 1.0,
            blank_penalty: 6.0,
        }
    }

    /// Create a config with custom temperature
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = temperature.max(0.1); // Prevent division by zero
        self
    }

    /// Create a config with custom blank penalty
    pub fn with_blank_penalty(mut self, blank_penalty: f32) -> Self {
        self.blank_penalty = blank_penalty.max(0.0).min(15.0);
        self
    }
}
