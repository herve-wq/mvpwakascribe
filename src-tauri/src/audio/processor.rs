use crate::error::{AppError, Result};
use rubato::{FftFixedInOut, Resampler};
use std::path::Path;
use tracing::info;

const TARGET_SAMPLE_RATE: u32 = 16000;

/// Resample audio to 16kHz mono
pub fn resample_to_16k(samples: &[f32], source_rate: u32) -> Result<Vec<f32>> {
    if source_rate == TARGET_SAMPLE_RATE {
        return Ok(samples.to_vec());
    }

    info!(
        "Resampling from {}Hz to {}Hz",
        source_rate, TARGET_SAMPLE_RATE
    );

    let mut resampler = FftFixedInOut::<f32>::new(
        source_rate as usize,
        TARGET_SAMPLE_RATE as usize,
        1024,
        1,
    )
    .map_err(|e| AppError::Audio(format!("Failed to create resampler: {}", e)))?;

    let chunk_size = resampler.input_frames_next();
    let mut output = Vec::new();

    for chunk in samples.chunks(chunk_size) {
        let mut input_chunk = chunk.to_vec();

        // Pad last chunk if needed
        if input_chunk.len() < chunk_size {
            input_chunk.resize(chunk_size, 0.0);
        }

        let result = resampler
            .process(&[input_chunk], None)
            .map_err(|e| AppError::Audio(format!("Resampling failed: {}", e)))?;

        if !result.is_empty() {
            output.extend(&result[0]);
        }
    }

    Ok(output)
}

/// Load audio from file and convert to 16kHz mono f32
pub fn load_audio_file(path: &Path) -> Result<(Vec<f32>, u32)> {
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_lowercase())
        .unwrap_or_default();

    match extension.as_str() {
        "wav" => load_wav(path),
        "mp3" | "m4a" | "ogg" | "flac" => {
            // For now, we only support WAV natively
            // Other formats would need additional dependencies like symphonia
            Err(AppError::Audio(format!(
                "Format {} not yet supported. Please convert to WAV.",
                extension
            )))
        }
        _ => Err(AppError::Audio(format!("Unknown audio format: {}", extension))),
    }
}

fn load_wav(path: &Path) -> Result<(Vec<f32>, u32)> {
    let reader = hound::WavReader::open(path).map_err(|e| AppError::Audio(e.to_string()))?;

    let spec = reader.spec();
    let sample_rate = spec.sample_rate;
    let channels = spec.channels as usize;

    info!(
        "Loading WAV: {}Hz, {} channels, {:?}",
        sample_rate, channels, spec.sample_format
    );

    let samples: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => reader
            .into_samples::<f32>()
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| AppError::Audio(e.to_string()))?,
        hound::SampleFormat::Int => {
            let bits = spec.bits_per_sample;
            let max_val = (1 << (bits - 1)) as f32;
            reader
                .into_samples::<i32>()
                .map(|s| s.map(|v| v as f32 / max_val))
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(|e| AppError::Audio(e.to_string()))?
        }
    };

    // Convert to mono by averaging channels
    let mono_samples: Vec<f32> = if channels > 1 {
        samples
            .chunks(channels)
            .map(|chunk| chunk.iter().sum::<f32>() / channels as f32)
            .collect()
    } else {
        samples
    };

    Ok((mono_samples, sample_rate))
}

/// Calculate the duration in milliseconds
pub fn duration_ms(samples: &[f32], sample_rate: u32) -> i64 {
    ((samples.len() as f64 / sample_rate as f64) * 1000.0) as i64
}
