//! Test binary for OpenVINO backend
//!
//! Run with: cargo run --bin test_openvino

use std::path::Path;
use wakascribe_lib::engine::{ASREngine, ParakeetEngine, TranscriptionLanguage, DecodingConfig};

fn main() {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .init();

    // Set OpenVINO library path
    std::env::set_var("OPENVINO_LIB_PATH", "/usr/local/lib");
    std::env::set_var("OV_LIB_PATH", "/usr/local/lib");
    std::env::set_var("DYLD_LIBRARY_PATH", "/usr/local/lib");

    println!("Testing OpenVINO backend...\n");

    // Model directory
    let model_dir = Path::new("/Users/herve/dev/mvpparakeet/wakascribe/model/openvino");
    if !model_dir.exists() {
        eprintln!("Model directory not found: {:?}", model_dir);
        std::process::exit(1);
    }
    println!("Model directory: {:?}", model_dir);

    // Create engine
    let mut engine = ParakeetEngine::new();

    // Load model
    println!("\n[1/2] Loading OpenVINO models...");
    match engine.load_model(model_dir) {
        Ok(_) => println!("✓ Models loaded successfully"),
        Err(e) => {
            eprintln!("✗ Failed to load models: {}", e);
            std::process::exit(1);
        }
    }

    // Load test audio
    println!("\n[2/2] Testing inference...");
    let test_audio_path = Path::new("/Users/herve/dev/mvpparakeet/wakascribe/model/test_audio.wav");

    // Read WAV file using hound
    let reader = hound::WavReader::open(test_audio_path).expect("Failed to open WAV");
    let spec = reader.spec();
    println!("Audio: {} Hz, {} channels, {} bits", spec.sample_rate, spec.channels, spec.bits_per_sample);

    // Convert to f32 mono
    let samples: Vec<f32> = reader
        .into_samples::<i16>()
        .filter_map(Result::ok)
        .map(|s| s as f32 / 32768.0)
        .collect();

    // Normalize audio (target RMS = 0.15)
    let rms: f32 = (samples.iter().map(|&s| s * s).sum::<f32>() / samples.len() as f32).sqrt();
    let target_rms = 0.15;
    let gain = if rms > 0.0001 { target_rms / rms } else { 1.0 };
    let normalized: Vec<f32> = samples.iter().map(|&s| (s * gain).clamp(-1.0, 1.0)).collect();

    println!("Audio: {} samples ({:.2}s), normalized with gain {:.1}x",
             normalized.len(), normalized.len() as f32 / 16000.0, gain);

    println!("\n=== TARGET: ===");
    println!("Je fais un premier test un deux trois quatre cinq six, je vais faire un deuxième test sept huit neuf dix onze douze, je fais un troisième test treize quatorze quinze seize, fin des tests.\n");

    // Test configurations - 10 tests with varying parameters
    let configs: Vec<(&str, DecodingConfig, TranscriptionLanguage)> = vec![
        // Test 1: Baseline greedy
        ("Test 1: Greedy (beam=1, temp=1.0, blank=6) French",
         DecodingConfig::greedy(), TranscriptionLanguage::French),
        // Test 2: Lower temperature
        ("Test 2: Greedy + temp=0.7 French",
         DecodingConfig::greedy().with_temperature(0.7), TranscriptionLanguage::French),
        // Test 3: Beam search basic
        ("Test 3: Beam=5 + temp=1.0 French",
         DecodingConfig::beam_search(5), TranscriptionLanguage::French),
        // Test 4: Beam + lower temp
        ("Test 4: Beam=5 + temp=0.7 French",
         DecodingConfig::beam_search(5).with_temperature(0.7), TranscriptionLanguage::French),
        // Test 5: Higher beam
        ("Test 5: Beam=10 + temp=0.7 French",
         DecodingConfig::beam_search(10).with_temperature(0.7), TranscriptionLanguage::French),
        // Test 6: Higher blank penalty
        ("Test 6: Beam=5 + temp=0.7 + blank=8 French",
         DecodingConfig::beam_search(5).with_temperature(0.7).with_blank_penalty(8.0), TranscriptionLanguage::French),
        // Test 7: Lower blank penalty
        ("Test 7: Beam=5 + temp=0.7 + blank=4 French",
         DecodingConfig::beam_search(5).with_temperature(0.7).with_blank_penalty(4.0), TranscriptionLanguage::French),
        // Test 8: Very low temperature
        ("Test 8: Beam=5 + temp=0.5 French",
         DecodingConfig::beam_search(5).with_temperature(0.5), TranscriptionLanguage::French),
        // Test 9: Auto language detection
        ("Test 9: Beam=5 + temp=0.7 Auto",
         DecodingConfig::beam_search(5).with_temperature(0.7), TranscriptionLanguage::Auto),
        // Test 10: High beam + balanced settings
        ("Test 10: Beam=10 + temp=0.8 + blank=7 French",
         DecodingConfig::beam_search(10).with_temperature(0.8).with_blank_penalty(7.0), TranscriptionLanguage::French),
    ];

    for (name, config, language) in &configs {
        println!("=== {} ===", name);

        let start = std::time::Instant::now();
        match engine.run_inference(&normalized, language.clone(), config) {
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

    println!("\n✓ OpenVINO test completed!");
}
