//! Test binary for ONNX Runtime backend
//!
//! Run with: cargo run --bin test_onnxruntime

use std::path::Path;
use wakascribe_lib::engine::{ASREngine, OnnxRuntimeEngine, TranscriptionLanguage, DecodingConfig};

fn main() {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    println!("Testing ONNX Runtime backend...\n");

    // Model directory
    let model_dir = Path::new("../model/onnxruntime");
    if !model_dir.exists() {
        eprintln!("Model directory not found: {:?}", model_dir);
        std::process::exit(1);
    }
    println!("Model directory: {:?}", model_dir);

    // Create engine
    let mut engine = OnnxRuntimeEngine::new();

    // Load model
    println!("\n[1/2] Loading ONNX models...");
    match engine.load_model(model_dir) {
        Ok(_) => println!("✓ Models loaded successfully"),
        Err(e) => {
            eprintln!("✗ Failed to load models: {}", e);
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
    println!("Audio: {} Hz, {} channels, {} bits", spec.sample_rate, spec.channels, spec.bits_per_sample);

    // Convert to f32 mono
    let samples: Vec<f32> = reader
        .into_samples::<i16>()
        .filter_map(Result::ok)
        .map(|s| s as f32 / 32768.0)
        .collect();

    println!("Loaded {} samples ({:.2}s)", samples.len(), samples.len() as f32 / 16000.0);

    // Resample if needed (expecting 16kHz)
    let samples = if spec.sample_rate != 16000 {
        println!("Note: Audio needs resampling from {} Hz to 16000 Hz", spec.sample_rate);
        // For now, just use as-is (would need rubato for proper resampling)
        samples
    } else {
        samples
    };

    // Test multiple configurations
    let configs = [
        ("Default (temp=1.0, blank=6.0)", DecodingConfig::default()),
        ("Higher blank penalty (temp=1.0, blank=8.0)", DecodingConfig::default().with_blank_penalty(8.0)),
        ("Higher blank penalty (temp=1.0, blank=10.0)", DecodingConfig::default().with_blank_penalty(10.0)),
        ("Temp 0.8 + blank 7.0", DecodingConfig::default().with_temperature(0.8).with_blank_penalty(7.0)),
        ("Temp 0.9 + blank 6.5", DecodingConfig::default().with_temperature(0.9).with_blank_penalty(6.5)),
    ];

    for (name, config) in &configs {
        println!("\n=== Testing: {} ===", name);
        println!("Config: beam_width={}, temperature={:.2}, blank_penalty={:.1}",
            config.beam_width, config.temperature, config.blank_penalty);

        match engine.run_inference(&samples, TranscriptionLanguage::French, config) {
            Ok(text) => {
                println!("Result: {}", text);
            }
            Err(e) => {
                eprintln!("✗ Inference failed: {}", e);
            }
        }
    }

    println!("\n✓ ONNX Runtime test completed successfully!");
}
