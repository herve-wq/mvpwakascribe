use ndarray::Array2;
use rustfft::{num_complex::Complex, FftPlanner};
use std::f32::consts::PI;

/// Mel spectrogram configuration
pub struct MelConfig {
    pub sample_rate: u32,
    pub n_fft: usize,
    pub hop_length: usize,
    pub n_mels: usize,
    pub fmin: f32,
    pub fmax: f32,
}

impl Default for MelConfig {
    fn default() -> Self {
        Self {
            sample_rate: 16000,
            n_fft: 512,
            hop_length: 160, // 10ms at 16kHz
            n_mels: 128,     // Parakeet uses 128 mel features
            fmin: 0.0,
            fmax: 8000.0,
        }
    }
}

/// Compute mel spectrogram from audio samples
pub fn compute_mel_spectrogram(samples: &[f32], config: &MelConfig) -> Array2<f32> {
    let n_fft = config.n_fft;
    let hop_length = config.hop_length;
    let n_mels = config.n_mels;

    // Create Hann window
    let window: Vec<f32> = (0..n_fft)
        .map(|i| 0.5 * (1.0 - (2.0 * PI * i as f32 / n_fft as f32).cos()))
        .collect();

    // Pad signal
    let pad_length = n_fft / 2;
    let mut padded = vec![0.0f32; pad_length];
    padded.extend_from_slice(samples);
    padded.extend(vec![0.0f32; pad_length]);

    // Compute STFT
    let num_frames = (padded.len() - n_fft) / hop_length + 1;
    let mut planner = FftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(n_fft);

    let mut spectrogram = Array2::<f32>::zeros((n_fft / 2 + 1, num_frames));

    for (frame_idx, start) in (0..padded.len() - n_fft + 1)
        .step_by(hop_length)
        .enumerate()
    {
        if frame_idx >= num_frames {
            break;
        }

        // Apply window and create complex buffer
        let mut buffer: Vec<Complex<f32>> = padded[start..start + n_fft]
            .iter()
            .zip(window.iter())
            .map(|(&s, &w)| Complex::new(s * w, 0.0))
            .collect();

        // Compute FFT
        fft.process(&mut buffer);

        // Compute magnitude spectrum (power spectrum)
        for (i, c) in buffer.iter().take(n_fft / 2 + 1).enumerate() {
            spectrogram[[i, frame_idx]] = c.norm_sqr();
        }
    }

    // Create mel filterbank
    let mel_filterbank = create_mel_filterbank(
        config.sample_rate,
        n_fft,
        n_mels,
        config.fmin,
        config.fmax,
    );

    // Apply mel filterbank
    let mel_spec = mel_filterbank.dot(&spectrogram);

    // Apply log with small epsilon for numerical stability
    mel_spec.mapv(|x| (x + 1e-10).ln())
}

/// Convert frequency to mel scale
fn hz_to_mel(hz: f32) -> f32 {
    2595.0 * (1.0 + hz / 700.0).log10()
}

/// Convert mel to frequency
fn mel_to_hz(mel: f32) -> f32 {
    700.0 * (10.0_f32.powf(mel / 2595.0) - 1.0)
}

/// Create mel filterbank matrix
fn create_mel_filterbank(
    sample_rate: u32,
    n_fft: usize,
    n_mels: usize,
    fmin: f32,
    fmax: f32,
) -> Array2<f32> {
    let n_freqs = n_fft / 2 + 1;

    // Mel points
    let mel_min = hz_to_mel(fmin);
    let mel_max = hz_to_mel(fmax);

    let mel_points: Vec<f32> = (0..n_mels + 2)
        .map(|i| mel_min + (mel_max - mel_min) * i as f32 / (n_mels + 1) as f32)
        .collect();

    // Convert to Hz
    let hz_points: Vec<f32> = mel_points.iter().map(|&m| mel_to_hz(m)).collect();

    // Convert to FFT bin indices
    let bin_points: Vec<usize> = hz_points
        .iter()
        .map(|&hz| ((n_fft + 1) as f32 * hz / sample_rate as f32).floor() as usize)
        .collect();

    // Create filterbank
    let mut filterbank = Array2::<f32>::zeros((n_mels, n_freqs));

    for m in 0..n_mels {
        let f_m_minus = bin_points[m];
        let f_m = bin_points[m + 1];
        let f_m_plus = bin_points[m + 2];

        // Rising slope
        for k in f_m_minus..f_m {
            if k < n_freqs {
                filterbank[[m, k]] = (k - f_m_minus) as f32 / (f_m - f_m_minus).max(1) as f32;
            }
        }

        // Falling slope
        for k in f_m..f_m_plus {
            if k < n_freqs {
                filterbank[[m, k]] = (f_m_plus - k) as f32 / (f_m_plus - f_m).max(1) as f32;
            }
        }
    }

    filterbank
}

/// Normalize mel spectrogram (per-feature normalization)
pub fn normalize_mel(mel_spec: &Array2<f32>) -> Array2<f32> {
    let mean = mel_spec.mean().unwrap_or(0.0);
    let std = mel_spec.std(0.0);
    let std = if std < 1e-6 { 1.0 } else { std };

    mel_spec.mapv(|x| (x - mean) / std)
}
