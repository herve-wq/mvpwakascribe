//! Test binary for CoreML sidecar backend
//!
//! Run with: cargo run --bin test_coreml

#[cfg(target_os = "macos")]
use wakascribe_lib::engine::{ASREngine, CoreMLEngine, DecodingConfig, TranscriptionLanguage};

use std::path::Path;

#[cfg(target_os = "macos")]
fn main() {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    println!("Testing CoreML sidecar backend...\n");

    // Model directory
    let model_dir = Path::new("../model/coreml");
    if !model_dir.exists() {
        eprintln!("Model directory not found: {:?}", model_dir);
        std::process::exit(1);
    }
    println!("Model directory: {:?}", model_dir);

    // Create engine
    let mut engine = CoreMLEngine::new();

    // Load model (this just verifies the sidecar exists)
    println!("\n[1/2] Initializing CoreML sidecar engine...");
    match engine.load_model(model_dir) {
        Ok(_) => println!("✓ Sidecar engine ready"),
        Err(e) => {
            eprintln!("✗ Failed to initialize: {}", e);
            std::process::exit(1);
        }
    }

    // Load test audio
    println!("\n[2/2] Testing inference...");
    let test_audio_path = Path::new("../model/test_audio.wav");
    if !test_audio_path.exists() {
        eprintln!("Test audio not found: {:?}", test_audio_path);
        std::process::exit(1);
    }

    // Read WAV file
    let reader = hound::WavReader::open(test_audio_path).expect("Failed to open WAV");
    let spec = reader.spec();
    println!(
        "Audio: {} Hz, {} channels, {} bits",
        spec.sample_rate, spec.channels, spec.bits_per_sample
    );

    // Convert to f32 mono
    let samples: Vec<f32> = reader
        .into_samples::<i16>()
        .filter_map(Result::ok)
        .map(|s| s as f32 / 32768.0)
        .collect();

    println!(
        "Loaded {} samples ({:.2}s)",
        samples.len(),
        samples.len() as f32 / 16000.0
    );

    // Run inference
    let config = DecodingConfig::default();
    println!("\nRunning inference...");

    let start = std::time::Instant::now();
    match engine.run_inference(&samples, TranscriptionLanguage::French, &config) {
        Ok(text) => {
            let elapsed = start.elapsed();
            println!("\n✓ Result: {}", text);
            println!(
                "Time: {:.2}s (RTF: {:.1}x)",
                elapsed.as_secs_f32(),
                (samples.len() as f32 / 16000.0) / elapsed.as_secs_f32()
            );
        }
        Err(e) => {
            eprintln!("✗ Inference failed: {}", e);
            std::process::exit(1);
        }
    }

    println!("\n✓ CoreML sidecar test completed successfully!");
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("CoreML is only available on macOS");
    std::process::exit(1);
}
