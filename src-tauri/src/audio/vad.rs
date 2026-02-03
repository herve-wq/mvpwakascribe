//! Voice Activity Detection (VAD) module
//!
//! Simple energy-based VAD for finding silence points in audio.
//! Used to split audio at natural pauses instead of mid-word.

use tracing::debug;

/// Sample rate (fixed at 16kHz for Parakeet)
const SAMPLE_RATE: usize = 16000;

/// Configuration for VAD
#[derive(Debug, Clone)]
pub struct VadConfig {
    /// Window size in samples for energy calculation
    pub window_samples: usize,
    /// Step size between windows in samples
    pub step_samples: usize,
    /// RMS threshold below which audio is considered silence
    pub silence_threshold: f32,
}

impl Default for VadConfig {
    fn default() -> Self {
        Self {
            window_samples: (0.1 * SAMPLE_RATE as f32) as usize, // 100ms window
            step_samples: (0.05 * SAMPLE_RATE as f32) as usize,  // 50ms step
            silence_threshold: 0.01,                              // RMS < 0.01 = silence
        }
    }
}

impl VadConfig {
    /// Create a VAD config with custom silence threshold
    pub fn with_threshold(silence_threshold: f32) -> Self {
        Self {
            silence_threshold,
            ..Default::default()
        }
    }
}

/// Result of VAD analysis for a segment
#[derive(Debug, Clone)]
pub struct VadFrame {
    /// Start position in samples
    pub start_sample: usize,
    /// End position in samples
    pub end_sample: usize,
    /// RMS energy of this frame
    pub rms: f32,
    /// Is this frame considered silence?
    pub is_silence: bool,
}

/// Analyze audio and return VAD frames
///
/// # Arguments
/// * `samples` - Audio samples at 16kHz
/// * `config` - VAD configuration
///
/// # Returns
/// Vector of VadFrame with energy information
pub fn analyze_audio(samples: &[f32], config: &VadConfig) -> Vec<VadFrame> {
    let mut frames = Vec::new();
    let mut pos = 0;

    while pos + config.window_samples <= samples.len() {
        let window = &samples[pos..pos + config.window_samples];
        let rms = compute_rms(window);
        let is_silence = rms < config.silence_threshold;

        frames.push(VadFrame {
            start_sample: pos,
            end_sample: pos + config.window_samples,
            rms,
            is_silence,
        });

        pos += config.step_samples;
    }

    frames
}

/// Find the best silence point in a range of samples
///
/// Returns the sample position with minimum energy (best cut point).
/// If no silence is found, returns the position with minimum energy.
///
/// # Arguments
/// * `samples` - Audio samples at 16kHz
/// * `search_start` - Start of search range (samples)
/// * `search_end` - End of search range (samples)
/// * `config` - VAD configuration
///
/// # Returns
/// (sample_position, rms_at_position, is_silence)
pub fn find_best_cut_point(
    samples: &[f32],
    search_start: usize,
    search_end: usize,
    config: &VadConfig,
) -> (usize, f32, bool) {
    let search_start = search_start.min(samples.len());
    let search_end = search_end.min(samples.len());

    if search_start >= search_end {
        return (search_start, 0.0, true);
    }

    let mut best_pos = search_start;
    let mut best_rms = f32::MAX;
    let mut found_silence = false;

    let mut pos = search_start;
    while pos + config.window_samples <= search_end {
        let window = &samples[pos..pos + config.window_samples];
        let rms = compute_rms(window);

        // Prefer silence points
        if rms < config.silence_threshold {
            if !found_silence || rms < best_rms {
                best_pos = pos + config.window_samples / 2; // Center of window
                best_rms = rms;
                found_silence = true;
            }
        } else if !found_silence && rms < best_rms {
            // No silence found yet, track minimum energy
            best_pos = pos + config.window_samples / 2;
            best_rms = rms;
        }

        pos += config.step_samples;
    }

    debug!(
        "Best cut point at {:.2}s: RMS={:.4}, silence={}",
        best_pos as f32 / SAMPLE_RATE as f32,
        best_rms,
        found_silence
    );

    (best_pos, best_rms, found_silence)
}

/// Find all silence regions in audio
///
/// # Arguments
/// * `samples` - Audio samples at 16kHz
/// * `config` - VAD configuration
///
/// # Returns
/// Vector of (start_sample, end_sample) for each silence region
pub fn find_silence_regions(samples: &[f32], config: &VadConfig) -> Vec<(usize, usize)> {
    let frames = analyze_audio(samples, config);
    let mut regions = Vec::new();
    let mut in_silence = false;
    let mut silence_start = 0;

    for frame in &frames {
        if frame.is_silence && !in_silence {
            // Start of silence region
            silence_start = frame.start_sample;
            in_silence = true;
        } else if !frame.is_silence && in_silence {
            // End of silence region
            regions.push((silence_start, frame.start_sample));
            in_silence = false;
        }
    }

    // Handle trailing silence
    if in_silence {
        regions.push((silence_start, samples.len()));
    }

    regions
}

/// Compute RMS (Root Mean Square) energy of audio samples
fn compute_rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum_sq: f64 = samples.iter().map(|&s| (s as f64) * (s as f64)).sum();
    (sum_sq / samples.len() as f64).sqrt() as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_silence_detection() {
        // Create audio with silence in the middle
        let mut samples = vec![0.5f32; 16000]; // 1s of loud audio
        samples.extend(vec![0.001f32; 8000]);  // 0.5s of silence
        samples.extend(vec![0.5f32; 16000]);   // 1s of loud audio

        let config = VadConfig::default();
        let frames = analyze_audio(&samples, &config);

        // Check that we detect some silence frames
        let silence_count = frames.iter().filter(|f| f.is_silence).count();
        assert!(silence_count > 0, "Should detect silence frames");
    }

    #[test]
    fn test_find_best_cut_point() {
        // Create audio with a clear silence point
        let mut samples = vec![0.5f32; 16000]; // 1s loud
        samples.extend(vec![0.001f32; 3200]);  // 0.2s silence at 1.0s
        samples.extend(vec![0.5f32; 16000]);   // 1s loud

        let config = VadConfig::default();
        let (pos, _rms, is_silence) = find_best_cut_point(
            &samples,
            8000,  // Search from 0.5s
            24000, // To 1.5s
            &config,
        );

        // Should find the silence point around 1.0-1.2s (16000-19200 samples)
        assert!(pos >= 15000 && pos <= 20000, "Should find cut point near silence");
        assert!(is_silence, "Should identify it as silence");
    }
}
