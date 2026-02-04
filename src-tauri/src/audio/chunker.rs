//! Audio chunking for long audio files
//!
//! Provides two chunking strategies:
//! 1. Fixed overlap chunking (legacy)
//! 2. Smart VAD-based chunking (recommended) - cuts at silence points

use super::vad::{find_best_cut_point, VadConfig};
use tracing::info;

/// Sample rate (fixed at 16kHz for Parakeet)
const SAMPLE_RATE: usize = 16000;

/// Maximum chunk duration in seconds (model limit is ~15s)
const MAX_CHUNK_SECONDS: f32 = 14.0;

/// A chunk of audio with metadata
#[derive(Debug, Clone)]
pub struct AudioChunk {
    /// The audio samples
    pub samples: Vec<f32>,
    /// Start time in milliseconds (in the original audio)
    pub start_ms: i64,
    /// End time in milliseconds (in the original audio)
    pub end_ms: i64,
    /// Chunk index (0-based)
    pub index: usize,
    /// Total number of chunks
    pub total_chunks: usize,
}

/// Configuration for smart VAD-based chunking
#[derive(Debug, Clone)]
pub struct SmartChunkConfig {
    /// Minimum chunk duration in seconds
    pub min_chunk_seconds: f32,
    /// Target chunk duration in seconds (preferred)
    pub target_chunk_seconds: f32,
    /// Maximum chunk duration in seconds (hard limit)
    pub max_chunk_seconds: f32,
    /// Overlap in seconds between chunks (to avoid losing content at boundaries)
    pub overlap_seconds: f32,
    /// VAD configuration for silence detection
    pub vad_config: VadConfig,
}

impl Default for SmartChunkConfig {
    fn default() -> Self {
        Self {
            min_chunk_seconds: 8.0,
            target_chunk_seconds: 10.0,
            max_chunk_seconds: MAX_CHUNK_SECONDS,
            overlap_seconds: 0.5, // 0.5 second overlap to capture boundary words
            vad_config: VadConfig::default(),
        }
    }
}

impl SmartChunkConfig {
    /// Create config with custom chunk durations
    pub fn new(min_seconds: f32, target_seconds: f32, max_seconds: f32) -> Self {
        Self {
            min_chunk_seconds: min_seconds,
            target_chunk_seconds: target_seconds,
            max_chunk_seconds: max_seconds.min(MAX_CHUNK_SECONDS),
            overlap_seconds: 0.5,
            vad_config: VadConfig::default(),
        }
    }

    fn min_samples(&self) -> usize {
        (self.min_chunk_seconds * SAMPLE_RATE as f32) as usize
    }

    fn target_samples(&self) -> usize {
        (self.target_chunk_seconds * SAMPLE_RATE as f32) as usize
    }

    fn max_samples(&self) -> usize {
        (self.max_chunk_seconds * SAMPLE_RATE as f32) as usize
    }

    fn overlap_samples(&self) -> usize {
        (self.overlap_seconds * SAMPLE_RATE as f32) as usize
    }
}

/// Split audio into chunks at silence points (VAD-based)
///
/// This is the recommended chunking strategy. Instead of cutting at fixed
/// intervals, it finds silence points between min and max duration to ensure
/// words are not cut in half.
///
/// # Arguments
/// * `samples` - Audio samples at 16kHz
/// * `config` - Smart chunking configuration
///
/// # Returns
/// Vector of AudioChunk with complete sentences/phrases
pub fn split_audio_smart(samples: &[f32], config: &SmartChunkConfig) -> Vec<AudioChunk> {
    let total_samples = samples.len();
    let total_duration = total_samples as f32 / SAMPLE_RATE as f32;

    // If audio fits in one chunk, return as-is
    if total_samples <= config.max_samples() {
        info!(
            "Audio fits in single chunk ({:.2}s)",
            total_duration
        );
        return vec![AudioChunk {
            samples: samples.to_vec(),
            start_ms: 0,
            end_ms: (total_duration * 1000.0) as i64,
            index: 0,
            total_chunks: 1,
        }];
    }

    let mut chunks = Vec::new();
    let mut chunk_start = 0;

    while chunk_start < total_samples {
        let remaining = total_samples - chunk_start;

        // If remaining audio fits in max chunk, take it all
        if remaining <= config.max_samples() {
            let chunk_samples = samples[chunk_start..].to_vec();
            let start_ms = (chunk_start as f64 / SAMPLE_RATE as f64 * 1000.0) as i64;
            let end_ms = (total_samples as f64 / SAMPLE_RATE as f64 * 1000.0) as i64;

            chunks.push(AudioChunk {
                samples: chunk_samples,
                start_ms,
                end_ms,
                index: chunks.len(),
                total_chunks: 0, // Will be updated later
            });
            break;
        }

        // Search for silence point between min and max duration
        let search_start = chunk_start + config.min_samples();
        let search_end = (chunk_start + config.max_samples()).min(total_samples);

        // Find best cut point (silence or minimum energy)
        let (cut_point, rms, is_silence) = find_best_cut_point(
            samples,
            search_start,
            search_end,
            &config.vad_config,
        );

        // Log cut decision
        let cut_time = cut_point as f32 / SAMPLE_RATE as f32;
        let chunk_duration = (cut_point - chunk_start) as f32 / SAMPLE_RATE as f32;

        // Only add overlap when NOT cutting at silence (to avoid word truncation)
        // When cutting at silence, there's no word to split, so no overlap needed
        let overlap = if is_silence {
            0
        } else {
            config.overlap_samples()
        };

        if is_silence {
            info!(
                "Chunk {}: cut at {:.2}s (silence, RMS={:.4}), duration={:.2}s, no overlap",
                chunks.len(),
                cut_time,
                rms,
                chunk_duration
            );
        } else {
            info!(
                "Chunk {}: cut at {:.2}s (min energy, RMS={:.4}), duration={:.2}s, overlap={:.1}s",
                chunks.len(),
                cut_time,
                rms,
                chunk_duration,
                config.overlap_seconds
            );
        }

        // Create chunk - only add overlap when NOT cutting at silence
        // When cutting at silence, the word boundary is clean
        let chunk_end = (cut_point + overlap).min(total_samples);
        let chunk_samples = samples[chunk_start..chunk_end].to_vec();
        let start_ms = (chunk_start as f64 / SAMPLE_RATE as f64 * 1000.0) as i64;
        let end_ms = (chunk_end as f64 / SAMPLE_RATE as f64 * 1000.0) as i64;

        chunks.push(AudioChunk {
            samples: chunk_samples,
            start_ms,
            end_ms,
            index: chunks.len(),
            total_chunks: 0,
        });

        // Move to next chunk starting at cut point
        // The overlap region [cut_point ... cut_point + overlap] is in current chunk
        // Next chunk will start fresh from cut_point, potentially capturing same content
        // The merger will deduplicate overlapping transcriptions
        chunk_start = cut_point;
    }

    // Update total_chunks
    let total_chunks = chunks.len();
    for chunk in &mut chunks {
        chunk.total_chunks = total_chunks;
    }

    info!(
        "Smart split: {:.2}s audio into {} chunks (min={:.1}s, target={:.1}s, max={:.1}s)",
        total_duration,
        total_chunks,
        config.min_chunk_seconds,
        config.target_chunk_seconds,
        config.max_chunk_seconds
    );

    chunks
}

// ============================================================================
// Legacy fixed-overlap chunking (kept for compatibility)
// ============================================================================

/// Default chunk size in seconds
pub const DEFAULT_CHUNK_SECONDS: f32 = 10.0;

/// Default overlap in seconds
pub const DEFAULT_OVERLAP_SECONDS: f32 = 2.0;

/// Configuration for fixed-overlap chunking (legacy)
#[derive(Debug, Clone)]
pub struct ChunkConfig {
    /// Chunk size in samples
    pub chunk_samples: usize,
    /// Overlap size in samples
    pub overlap_samples: usize,
}

impl Default for ChunkConfig {
    fn default() -> Self {
        Self {
            chunk_samples: (DEFAULT_CHUNK_SECONDS * SAMPLE_RATE as f32) as usize,
            overlap_samples: (DEFAULT_OVERLAP_SECONDS * SAMPLE_RATE as f32) as usize,
        }
    }
}

impl ChunkConfig {
    /// Create a new chunk config with custom parameters
    pub fn new(chunk_seconds: f32, overlap_seconds: f32) -> Self {
        Self {
            chunk_samples: (chunk_seconds * SAMPLE_RATE as f32) as usize,
            overlap_samples: (overlap_seconds * SAMPLE_RATE as f32) as usize,
        }
    }

    /// Step size between chunk starts (chunk_size - overlap)
    pub fn step_samples(&self) -> usize {
        self.chunk_samples.saturating_sub(self.overlap_samples)
    }
}

/// Split audio into overlapping chunks (legacy method)
///
/// **Note**: Prefer `split_audio_smart()` which cuts at silence points
/// to avoid cutting words in half.
///
/// # Arguments
/// * `samples` - Audio samples at 16kHz
/// * `config` - Chunking configuration
///
/// # Returns
/// Vector of AudioChunk with overlapping segments
pub fn split_audio(samples: &[f32], config: &ChunkConfig) -> Vec<AudioChunk> {
    let total_samples = samples.len();

    // If audio fits in one chunk, return as-is
    if total_samples <= config.chunk_samples {
        info!(
            "Audio fits in single chunk ({} samples, {:.2}s)",
            total_samples,
            total_samples as f32 / SAMPLE_RATE as f32
        );
        return vec![AudioChunk {
            samples: samples.to_vec(),
            start_ms: 0,
            end_ms: (total_samples as f64 / SAMPLE_RATE as f64 * 1000.0) as i64,
            index: 0,
            total_chunks: 1,
        }];
    }

    let step = config.step_samples();
    let mut chunks = Vec::new();

    // Calculate total number of chunks
    let total_chunks = ((total_samples as f32 - config.overlap_samples as f32) / step as f32)
        .ceil() as usize;

    let mut start = 0;
    let mut index = 0;

    while start < total_samples {
        let end = (start + config.chunk_samples).min(total_samples);
        let chunk_samples = samples[start..end].to_vec();

        let start_ms = (start as f64 / SAMPLE_RATE as f64 * 1000.0) as i64;
        let end_ms = (end as f64 / SAMPLE_RATE as f64 * 1000.0) as i64;

        chunks.push(AudioChunk {
            samples: chunk_samples,
            start_ms,
            end_ms,
            index,
            total_chunks,
        });

        // Move to next chunk
        start += step;
        index += 1;

        // Stop if we've covered all samples
        if end >= total_samples {
            break;
        }
    }

    // Update total_chunks in all chunks (we now know the actual count)
    let actual_total = chunks.len();
    for chunk in &mut chunks {
        chunk.total_chunks = actual_total;
    }

    info!(
        "Split {:.2}s audio into {} chunks (chunk={:.1}s, overlap={:.1}s, step={:.1}s)",
        total_samples as f32 / SAMPLE_RATE as f32,
        actual_total,
        config.chunk_samples as f32 / SAMPLE_RATE as f32,
        config.overlap_samples as f32 / SAMPLE_RATE as f32,
        step as f32 / SAMPLE_RATE as f32,
    );

    chunks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_short_audio_single_chunk() {
        let samples = vec![0.0f32; 16000 * 5]; // 5 seconds
        let config = SmartChunkConfig::default();
        let chunks = split_audio_smart(&samples, &config);

        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].start_ms, 0);
        assert_eq!(chunks[0].end_ms, 5000);
    }

    #[test]
    fn test_smart_chunking_finds_silence() {
        // Create 25s audio with silence at 10s and 20s
        let mut samples = Vec::new();

        // 0-10s: speech (high energy)
        samples.extend(vec![0.3f32; 16000 * 10]);
        // 10-10.2s: silence
        samples.extend(vec![0.001f32; 16000 / 5]);
        // 10.2-20s: speech
        samples.extend((0..16000 * 10 - 16000 / 5).map(|_| 0.3f32));
        // 20-20.2s: silence
        samples.extend(vec![0.001f32; 16000 / 5]);
        // 20.2-25s: speech
        samples.extend(vec![0.3f32; 16000 * 5 - 16000 / 5]);

        let config = SmartChunkConfig::default();
        let chunks = split_audio_smart(&samples, &config);

        // Should cut at silence points, creating 2-3 chunks
        assert!(chunks.len() >= 2 && chunks.len() <= 3);

        // First chunk should be around 10s (cut at first silence)
        let first_duration_s = chunks[0].samples.len() as f32 / 16000.0;
        assert!(
            first_duration_s >= 9.5 && first_duration_s <= 11.0,
            "First chunk should be ~10s, got {:.2}s",
            first_duration_s
        );
    }

    #[test]
    fn test_legacy_chunking_with_overlap() {
        // 25 seconds of audio
        let samples = vec![0.0f32; 16000 * 25];
        let config = ChunkConfig::new(10.0, 2.0);
        let chunks = split_audio(&samples, &config);

        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].start_ms, 0);
        assert_eq!(chunks[0].end_ms, 10000);
        assert_eq!(chunks[1].start_ms, 8000);
        assert_eq!(chunks[1].end_ms, 18000);
    }
}
