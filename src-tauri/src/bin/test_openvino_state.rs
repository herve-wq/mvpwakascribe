//! Test program to diagnose OpenVINO state accumulation bug in Rust bindings
//!
//! Run with: cargo run --bin test_openvino_state
//!
//! This tests the same scenarios as the Python script to compare behavior.

use openvino::{Core, DeviceType};
use std::path::Path;

// Use absolute path or set via env var
fn get_model_dir() -> String {
    std::env::var("MODEL_DIR").unwrap_or_else(|_| {
        // Try to find model relative to executable
        if let Ok(exe) = std::env::current_exe() {
            let mut path = exe;
            // Go up from target/debug/test_openvino_state to wakascribe, then into model
            for _ in 0..3 {
                path.pop();
            }
            path.push("model");
            if path.exists() {
                return path.to_string_lossy().to_string();
            }
        }
        // Fallback
        "/Users/herve/dev/mvpparakeet/wakascribe/model".to_string()
    })
}
const MEL_FEATURES: usize = 128;
const MAX_MEL_FRAMES: usize = 1501;
const MAX_AUDIO_SAMPLES: usize = 240000;

fn compute_rms(data: &[f32]) -> f32 {
    if data.is_empty() {
        return 0.0;
    }
    let sum_sq: f64 = data.iter().map(|&v| (v as f64) * (v as f64)).sum();
    (sum_sq / data.len() as f64).sqrt() as f32
}

fn generate_test_audio(seed: u64, amplitude: f32) -> Vec<f32> {
    // Simple deterministic pseudo-random generator
    let mut state = seed;
    let mut audio = Vec::with_capacity(48000);
    for _ in 0..48000 {
        state = state.wrapping_mul(1103515245).wrapping_add(12345);
        let random = ((state >> 16) & 0x7FFF) as f32 / 32767.0 * 2.0 - 1.0;
        audio.push(random * amplitude);
    }
    audio
}

fn run_mel_inference(
    mel_request: &mut openvino::InferRequest,
    audio: &[f32],
) -> Result<Vec<f32>, String> {
    // Pad audio
    let actual_len = audio.len().min(MAX_AUDIO_SAMPLES);
    let mut padded = vec![0.0f32; MAX_AUDIO_SAMPLES];
    padded[..actual_len].copy_from_slice(&audio[..actual_len]);

    // Set input
    let mut input_tensor = mel_request
        .get_tensor("input_signals")
        .map_err(|e| format!("get input tensor: {:?}", e))?;
    {
        let data = input_tensor
            .get_data_mut::<f32>()
            .map_err(|e| format!("get input data: {:?}", e))?;
        data[..MAX_AUDIO_SAMPLES].copy_from_slice(&padded);
    }

    let mut length_tensor = mel_request
        .get_tensor("input_length")
        .map_err(|e| format!("get length tensor: {:?}", e))?;
    length_tensor
        .get_data_mut::<i64>()
        .map_err(|e| format!("get length data: {:?}", e))?[0] = actual_len as i64;

    // Infer
    mel_request.infer().map_err(|e| format!("infer: {:?}", e))?;

    // Get output
    let output = mel_request
        .get_output_tensor_by_index(0)
        .map_err(|e| format!("get output: {:?}", e))?;
    let data = output
        .get_data::<f32>()
        .map_err(|e| format!("get output data: {:?}", e))?;

    Ok(data.to_vec())
}

fn run_encoder_inference(
    encoder_request: &mut openvino::InferRequest,
    mel_features: &[f32],
    valid_frames: usize,
) -> Result<(Vec<f32>, f32), String> {
    // Prepare input
    let mut input_tensor = encoder_request
        .get_tensor("melspectogram")
        .map_err(|e| format!("get input tensor: {:?}", e))?;
    {
        let data = input_tensor
            .get_data_mut::<f32>()
            .map_err(|e| format!("get input data: {:?}", e))?;
        data.fill(0.0);
        let copy_len = mel_features.len().min(data.len());
        data[..copy_len].copy_from_slice(&mel_features[..copy_len]);
    }

    let mut length_tensor = encoder_request
        .get_tensor("melspectogram_length")
        .map_err(|e| format!("get length tensor: {:?}", e))?;
    length_tensor
        .get_data_mut::<i32>()
        .map_err(|e| format!("get length data: {:?}", e))?[0] = valid_frames as i32;

    // Infer
    encoder_request
        .infer()
        .map_err(|e| format!("infer: {:?}", e))?;

    // Get output
    let output_tensor = encoder_request
        .get_tensor("encoder_output")
        .map_err(|e| format!("get output: {:?}", e))?;
    let output_data = output_tensor
        .get_data::<f32>()
        .map_err(|e| format!("get output data: {:?}", e))?;

    let rms = compute_rms(output_data);
    Ok((output_data.to_vec(), rms))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("OpenVINO State Accumulation Test (Rust)");
    println!("========================================\n");

    // Set OpenVINO library path
    std::env::set_var("OPENVINO_LIB_PATH", "/usr/local/lib");

    let model_dir_str = get_model_dir();
    let model_dir = Path::new(&model_dir_str);
    let encoder_xml = model_dir.join("parakeet_encoder.xml");
    let mel_xml = model_dir.join("parakeet_melspectogram.xml");

    if !encoder_xml.exists() {
        eprintln!("ERROR: Encoder model not found at {:?}", encoder_xml);
        eprintln!("Run from wakascribe/src-tauri directory");
        return Ok(());
    }

    println!("Loading models...");
    let mut core = Core::new()?;

    // Load mel model
    let mel_model_raw = core.read_model_from_file(mel_xml.to_str().unwrap(), "")?;
    let mut mel_compiled = core.compile_model(&mel_model_raw, DeviceType::CPU)?;
    let mut mel_request = mel_compiled.create_infer_request()?;

    // Load encoder model
    let encoder_model_raw = core.read_model_from_file(encoder_xml.to_str().unwrap(), "")?;
    let mut encoder_compiled = core.compile_model(&encoder_model_raw, DeviceType::CPU)?;

    println!("Models loaded.\n");

    // Generate test audio
    let audio = generate_test_audio(42, 0.1);
    println!("Test audio: {} samples, RMS={:.4}", audio.len(), compute_rms(&audio));

    // Compute mel features once
    let mel_features = run_mel_inference(&mut mel_request, &audio)?;
    let valid_frames = (audio.len() / 160).min(MAX_MEL_FRAMES);
    println!(
        "Mel features: {} elements, valid frames: {}\n",
        mel_features.len(),
        valid_frames
    );

    // ========================================
    // TEST 1: Same InferRequest, repeated inference
    // ========================================
    println!("TEST 1: Same InferRequest, repeated inference");
    println!("----------------------------------------------");
    {
        let mut request = encoder_compiled.create_infer_request()?;
        let mut rms_values = Vec::new();

        for i in 0..5 {
            let (_, rms) = run_encoder_inference(&mut request, &mel_features, valid_frames)?;
            rms_values.push(rms);
            println!("  Inference {}: RMS = {:.6}", i + 1, rms);
        }

        let drift = rms_values.iter().cloned().fold(f32::MIN, f32::max)
            - rms_values.iter().cloned().fold(f32::MAX, f32::min);
        println!("\n  RMS drift: {:.6}", drift);
        if drift > 0.001 {
            println!("  ❌ FAIL: State accumulates in same InferRequest");
        } else {
            println!("  ✅ PASS: No state accumulation");
        }
    }

    // ========================================
    // TEST 2: New InferRequest from same CompiledModel
    // ========================================
    println!("\nTEST 2: New InferRequest from same CompiledModel");
    println!("-------------------------------------------------");
    {
        let mut rms_values = Vec::new();

        for i in 0..5 {
            let mut request = encoder_compiled.create_infer_request()?;
            let (_, rms) = run_encoder_inference(&mut request, &mel_features, valid_frames)?;
            rms_values.push(rms);
            println!("  New request {}: RMS = {:.6}", i + 1, rms);
        }

        let drift = rms_values.iter().cloned().fold(f32::MIN, f32::max)
            - rms_values.iter().cloned().fold(f32::MAX, f32::min);
        println!("\n  RMS drift: {:.6}", drift);
        if drift > 0.001 {
            println!("  ❌ FAIL: State accumulates in CompiledModel");
        } else {
            println!("  ✅ PASS: New InferRequest creates fresh state");
        }
    }

    // ========================================
    // TEST 3: New CompiledModel each time
    // ========================================
    println!("\nTEST 3: New CompiledModel each time");
    println!("------------------------------------");
    {
        let mut rms_values = Vec::new();

        for i in 0..3 {
            // Reload model completely
            let model = core.read_model_from_file(encoder_xml.to_str().unwrap(), "")?;
            let mut compiled = core.compile_model(&model, DeviceType::CPU)?;
            let mut request = compiled.create_infer_request()?;

            let (_, rms) = run_encoder_inference(&mut request, &mel_features, valid_frames)?;
            rms_values.push(rms);
            println!("  Fresh model {}: RMS = {:.6}", i + 1, rms);
        }

        let drift = rms_values.iter().cloned().fold(f32::MIN, f32::max)
            - rms_values.iter().cloned().fold(f32::MAX, f32::min);
        println!("\n  RMS drift: {:.6}", drift);
        if drift > 0.001 {
            println!("  ❌ FAIL: Even fresh CompiledModel has drift");
        } else {
            println!("  ✅ PASS: Fresh CompiledModel is clean");
        }
    }

    // ========================================
    // TEST 4: Different audio, then repeat first
    // ========================================
    println!("\nTEST 4: Different audio, then repeat first");
    println!("-------------------------------------------");
    {
        let mut request = encoder_compiled.create_infer_request()?;

        // First audio
        let (_, rms1) = run_encoder_inference(&mut request, &mel_features, valid_frames)?;
        println!("  Audio seed=42: RMS = {:.6}", rms1);

        // Different audio
        let audio2 = generate_test_audio(123, 0.1);
        let mel2 = run_mel_inference(&mut mel_request, &audio2)?;
        let vf2 = (audio2.len() / 160).min(MAX_MEL_FRAMES);
        let (_, rms2) = run_encoder_inference(&mut request, &mel2, vf2)?;
        println!("  Audio seed=123: RMS = {:.6}", rms2);

        // Third audio
        let audio3 = generate_test_audio(456, 0.1);
        let mel3 = run_mel_inference(&mut mel_request, &audio3)?;
        let vf3 = (audio3.len() / 160).min(MAX_MEL_FRAMES);
        let (_, rms3) = run_encoder_inference(&mut request, &mel3, vf3)?;
        println!("  Audio seed=456: RMS = {:.6}", rms3);

        // Repeat first audio
        let (_, rms1_repeat) = run_encoder_inference(&mut request, &mel_features, valid_frames)?;
        println!("  Audio seed=42 (repeat): RMS = {:.6}", rms1_repeat);

        let drift = (rms1 - rms1_repeat).abs();
        println!("\n  RMS drift on repeat: {:.6}", drift);
        if drift > 0.001 {
            println!("  ❌ FAIL: Same input produces different output after other inferences");
        } else {
            println!("  ✅ PASS: Same input produces same output");
        }
    }

    // ========================================
    // SUMMARY
    // ========================================
    println!("\n========================================");
    println!("Compare these results with Python test!");
    println!("========================================");

    Ok(())
}
