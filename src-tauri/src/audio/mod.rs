pub mod capture;
pub mod chunker;
pub mod processor;
pub mod vad;

pub use capture::AudioCapture;
pub use chunker::{split_audio_smart, SmartChunkConfig};
pub use processor::{duration_ms, load_audio_file, normalize_audio, resample_to_16k, write_wav};

use crate::error::Result;

/// Resample to 16kHz and normalize audio in one step
pub fn prepare_audio(samples: &[f32], sample_rate: u32) -> Result<Vec<f32>> {
    let resampled = resample_to_16k(samples, sample_rate)?;
    let (normalized, _gain) = normalize_audio(&resampled);
    Ok(normalized)
}
