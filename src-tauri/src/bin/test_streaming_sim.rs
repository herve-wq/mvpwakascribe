//! Simulate streaming transcription on a WAV file and compare with single-shot.
//!
//! Usage: cargo run --bin test_streaming_sim -- <path-to-wav> [language]
//!   language: french (default), english, auto

use std::path::{Path, PathBuf};
use std::time::Instant;

use wakascribe_lib::audio::vad::{find_best_cut_point, VadConfig};
use wakascribe_lib::audio::{
    calculate_rms, load_audio_file, normalize_audio, resample_to_16k, trim_trailing_silence,
};
use wakascribe_lib::engine::{
    filter_chunk_hallucinations, ASREngine, DecodingConfig, ParakeetEngine, TranscriptionLanguage,
};

/// Minimum samples to transcribe a chunk (3s at 16kHz)
const MIN_CHUNK_SAMPLES: usize = 48000;

/// Simulated drain interval (seconds of source audio per cycle)
const DRAIN_SECONDS: usize = 10;

/// Relative silence ratio: discard leftover / trim trailing if RMS < ratio × chunk RMS
const SILENCE_RATIO: f32 = 0.1;

fn get_model_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("MODEL_DIR") {
        return PathBuf::from(dir);
    }

    if let Ok(exe) = std::env::current_exe() {
        let mut path = exe;
        // src-tauri/target/debug/test_streaming_sim -> project root
        for _ in 0..4 {
            path.pop();
        }
        path.push("model");
        if path.exists() {
            return path;
        }
    }

    PathBuf::from("model")
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Init OpenVINO (same logic as the Tauri app)
    if !wakascribe_lib::init_openvino() {
        eprintln!("Warning: OpenVINO library not found in standard paths");
    }

    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <wav-file> [language]", args[0]);
        eprintln!("  language: french (default), english, auto");
        std::process::exit(1);
    }

    let wav_path = Path::new(&args[1]);
    let lang = if args.len() > 2 {
        match args[2].as_str() {
            "english" => TranscriptionLanguage::English,
            "auto" => TranscriptionLanguage::Auto,
            _ => TranscriptionLanguage::French,
        }
    } else {
        TranscriptionLanguage::French
    };
    let config = DecodingConfig::default();

    println!("=== Streaming Simulation Test ===");
    println!("File: {}", wav_path.display());
    println!("Language: {:?}", lang);
    println!(
        "Config: beam_width={}, temperature={:.1}, blank_penalty={:.1}\n",
        config.beam_width, config.temperature, config.blank_penalty
    );

    // Load engine
    let model_dir = get_model_dir();
    let openvino_dir = model_dir.join("openvino");
    println!("Loading OpenVINO model from {:?}...", openvino_dir);
    let mut engine = ParakeetEngine::new();
    engine.load_model(&openvino_dir)?;
    println!("Model loaded.\n");

    // Load audio
    let (samples, sample_rate) = load_audio_file(wav_path)?;
    let duration_s = samples.len() as f64 / sample_rate as f64;
    println!(
        "Audio: {:.1}s at {}Hz ({} samples)\n",
        duration_s,
        sample_rate,
        samples.len()
    );

    // Resample full audio to 16kHz once
    let full_16k = resample_to_16k(&samples, sample_rate)?;
    println!(
        "Resampled to 16kHz: {} samples ({:.1}s)\n",
        full_16k.len(),
        full_16k.len() as f64 / 16000.0
    );

    // ==========================================
    // PASS 1: Single-shot transcription
    // ==========================================
    println!("=== PASS 1: Single-shot ===");
    let (normalized_full, gain) = normalize_audio(&full_16k);
    println!(
        "Normalized: {} samples, gain={:.1}x",
        normalized_full.len(),
        gain
    );
    let start = Instant::now();
    let single_shot_text = engine.run_inference(&normalized_full, lang, &config)?;
    let elapsed = start.elapsed();
    println!("Result ({:.0}ms):\n  {}\n", elapsed.as_millis(), single_shot_text);

    // ==========================================
    // PASS 2: Simulated streaming
    // ==========================================
    println!("=== PASS 2: Simulated streaming ({}s drain cycles) ===\n", DRAIN_SECONDS);

    let drain_samples = DRAIN_SECONDS * sample_rate as usize;
    let vad_config = VadConfig::default();
    let mut leftover_16k: Vec<f32> = Vec::new();
    let mut segments: Vec<String> = Vec::new();
    let mut pos = 0;
    let mut chunk_num = 0;

    while pos < samples.len() {
        // Simulate drain: take DRAIN_SECONDS worth of source audio
        let end = (pos + drain_samples).min(samples.len());
        let drained = &samples[pos..end];
        pos = end;

        // Resample drained audio, prepend leftover
        let mut raw_16k = std::mem::take(&mut leftover_16k);
        let resampled = resample_to_16k(drained, sample_rate)?;
        raw_16k.extend_from_slice(&resampled);

        // Skip if < 3s
        if raw_16k.len() < MIN_CHUNK_SAMPLES {
            let len = raw_16k.len();
            leftover_16k = raw_16k;
            println!(
                "  (buffering: {:.2}s < 3s minimum, carry to next cycle)",
                len as f64 / 16000.0
            );
            continue;
        }

        chunk_num += 1;

        // VAD cut in last 3s
        let mut to_normalize;
        let search_margin = 16000 * 3;
        if raw_16k.len() > MIN_CHUNK_SAMPLES + search_margin {
            let search_start = raw_16k.len() - search_margin;
            let (cut, _rms, is_silence) =
                find_best_cut_point(&raw_16k, search_start, raw_16k.len(), &vad_config);

            to_normalize = raw_16k[..cut].to_vec();
            let candidate_leftover = &raw_16k[cut..];
            let chunk_rms = calculate_rms(&to_normalize);
            let leftover_rms = calculate_rms(candidate_leftover);
            let threshold = chunk_rms * SILENCE_RATIO;

            print!(
                "  Chunk {}: VAD cut at {:.2}s (silence={}), leftover {:.2}s RMS={:.4} (chunk={:.4}, thr={:.4})",
                chunk_num,
                cut as f64 / 16000.0,
                is_silence,
                candidate_leftover.len() as f64 / 16000.0,
                leftover_rms,
                chunk_rms,
                threshold
            );

            if leftover_rms >= threshold {
                leftover_16k = candidate_leftover.to_vec();
            } else {
                print!(" [DISCARDED]");
                leftover_16k = Vec::new();
            }
            println!();
        } else {
            println!(
                "  Chunk {}: no VAD cut ({:.2}s, too short for margin)",
                chunk_num,
                raw_16k.len() as f64 / 16000.0
            );
            to_normalize = raw_16k;
        }

        // Trim trailing silence
        let keep = trim_trailing_silence(&to_normalize, SILENCE_RATIO);
        if keep < to_normalize.len() {
            println!(
                "  Chunk {}: trimmed trailing silence {:.2}s -> {:.2}s",
                chunk_num,
                to_normalize.len() as f64 / 16000.0,
                keep as f64 / 16000.0
            );
            to_normalize.truncate(keep);
        }

        // Normalize
        let (to_infer, gain) = normalize_audio(&to_normalize);

        // Infer
        let start = Instant::now();
        let text = engine.run_inference(&to_infer, lang, &config)?;
        let elapsed = start.elapsed();
        let filtered = filter_chunk_hallucinations(&text);

        println!(
            "  Chunk {}: {:.2}s audio, gain={:.1}x -> ({:.0}ms)",
            chunk_num,
            to_infer.len() as f64 / 16000.0,
            gain,
            elapsed.as_millis()
        );
        if filtered != text {
            println!("    raw:      '{}'", text);
            println!("    filtered: '{}'", filtered);
        } else {
            println!("    text: '{}'", text);
        }
        println!();

        if !filtered.is_empty() {
            segments.push(filtered);
        }
    }

    // Process remaining leftover
    if !leftover_16k.is_empty() && leftover_16k.len() > 16000 {
        chunk_num += 1;
        let keep = trim_trailing_silence(&leftover_16k, SILENCE_RATIO);
        if keep > 16000 {
            let (to_infer, gain) = normalize_audio(&leftover_16k[..keep]);
            let start = Instant::now();
            let text = engine.run_inference(&to_infer, lang, &config)?;
            let elapsed = start.elapsed();
            let filtered = filter_chunk_hallucinations(&text);

            println!(
                "  Chunk {} (final leftover): {:.2}s, gain={:.1}x -> ({:.0}ms)",
                chunk_num,
                to_infer.len() as f64 / 16000.0,
                gain,
                elapsed.as_millis()
            );
            if filtered != text {
                println!("    raw:      '{}'", text);
                println!("    filtered: '{}'", filtered);
            } else {
                println!("    text: '{}'", text);
            }
            println!();

            if !filtered.is_empty() {
                segments.push(filtered);
            }
        } else {
            println!(
                "  Final leftover too short after trim ({:.2}s), skipped",
                keep as f64 / 16000.0
            );
        }
    } else if !leftover_16k.is_empty() {
        println!(
            "  Final leftover too short ({:.2}s < 1s), skipped",
            leftover_16k.len() as f64 / 16000.0
        );
    }

    let streaming_text = segments.join(" ");

    // ==========================================
    // COMPARISON
    // ==========================================
    println!("=== COMPARISON ===");
    println!("Single-shot ({} chars):", single_shot_text.len());
    println!("  {}", single_shot_text);
    println!();
    println!("Streaming ({} chars, {} chunks):", streaming_text.len(), segments.len());
    println!("  {}", streaming_text);
    println!();

    // Per-segment breakdown
    println!("=== SEGMENTS ===");
    for (i, seg) in segments.iter().enumerate() {
        println!("  [{}] {}", i + 1, seg);
    }

    Ok(())
}
