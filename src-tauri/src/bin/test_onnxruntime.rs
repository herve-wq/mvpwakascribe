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

    println!("\n=== TARGET (CoreML FluidAudio): ===");
    println!("Je fais un premier test. 1, 2, 3, 4, 5, 6. Je vais faire un deuxième test. 7, 8, 9, 10, 11, 12. Je fais un troisième test. 13, 14, 15, 16. Fin des tests.\n");

    // Test configurations (max 10 tests) - Round 2: Fine-tuning around beam=5, temp=0.7
    let configs: Vec<(&str, DecodingConfig, TranscriptionLanguage)> = vec![
        // Test 1: Best from round 1
        ("Test 1: Beam=5 + temp=0.7 + blank=6 Auto",
         DecodingConfig::beam_search(5).with_temperature(0.7), TranscriptionLanguage::Auto),
        // Test 2: Same but French
        ("Test 2: Beam=5 + temp=0.7 + blank=6 French",
         DecodingConfig::beam_search(5).with_temperature(0.7), TranscriptionLanguage::French),
        // Test 3: Higher beam
        ("Test 3: Beam=8 + temp=0.7 + blank=6 Auto",
         DecodingConfig::beam_search(8).with_temperature(0.7), TranscriptionLanguage::Auto),
        // Test 4: Beam=5 + higher blank
        ("Test 4: Beam=5 + temp=0.7 + blank=8 Auto",
         DecodingConfig::beam_search(5).with_temperature(0.7).with_blank_penalty(8.0), TranscriptionLanguage::Auto),
        // Test 5: Beam=5 + temp=0.8
        ("Test 5: Beam=5 + temp=0.8 + blank=6 Auto",
         DecodingConfig::beam_search(5).with_temperature(0.8), TranscriptionLanguage::Auto),
        // Test 6: Beam=5 + temp=0.6
        ("Test 6: Beam=5 + temp=0.6 + blank=6 Auto",
         DecodingConfig::beam_search(5).with_temperature(0.6), TranscriptionLanguage::Auto),
        // Test 7: Beam=5 + temp=0.7 + blank=7
        ("Test 7: Beam=5 + temp=0.7 + blank=7 Auto",
         DecodingConfig::beam_search(5).with_temperature(0.7).with_blank_penalty(7.0), TranscriptionLanguage::Auto),
        // Test 8: Beam=10 + temp=0.7
        ("Test 8: Beam=10 + temp=0.7 + blank=6 Auto",
         DecodingConfig::beam_search(10).with_temperature(0.7), TranscriptionLanguage::Auto),
        // Test 9: Beam=5 + temp=0.75 + blank=7 French
        ("Test 9: Beam=5 + temp=0.75 + blank=7 French",
         DecodingConfig::beam_search(5).with_temperature(0.75).with_blank_penalty(7.0), TranscriptionLanguage::French),
        // Test 10: Beam=7 + temp=0.7 + blank=7 Auto
        ("Test 10: Beam=7 + temp=0.7 + blank=7 Auto",
         DecodingConfig::beam_search(7).with_temperature(0.7).with_blank_penalty(7.0), TranscriptionLanguage::Auto),
    ];

    for (name, config, language) in &configs {
        println!("=== {} ===", name);

        let start = std::time::Instant::now();
        match engine.run_inference(&samples, language.clone(), config) {
            Ok(text) => {
                let elapsed = start.elapsed();
                println!("Result: {}", text);
                println!("Time: {:?}\n", elapsed);
            }
            Err(e) => {
                eprintln!("✗ Inference failed: {}\n", e);
            }
        }
    }

    println!("\n✓ ONNX Runtime test completed successfully!");
}
