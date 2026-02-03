//! Test the FULL pipeline exactly like parakeet.rs to reproduce the bug
//!
//! Run with: cargo run --bin test_full_pipeline

use openvino::{CompiledModel, Core, DeviceType, InferRequest};
use std::path::Path;
use std::sync::Mutex;

const MAX_AUDIO_SAMPLES: usize = 240000;
const MAX_MEL_FRAMES: usize = 1501;
const MEL_FEATURES: usize = 128;
const ENCODER_OUTPUT_DIM: usize = 1024;
const MAX_ENCODER_TIME: usize = 188;
const DECODER_HIDDEN_DIM: usize = 640;
const DECODER_NUM_LAYERS: usize = 2;
const HOP_LENGTH: usize = 160;

fn get_model_dir() -> String {
    std::env::var("MODEL_DIR").unwrap_or_else(|_| {
        if let Ok(exe) = std::env::current_exe() {
            let mut path = exe;
            for _ in 0..3 {
                path.pop();
            }
            path.push("model");
            if path.exists() {
                return path.to_string_lossy().to_string();
            }
        }
        "/Users/herve/dev/mvpparakeet/wakascribe/model".to_string()
    })
}

fn compute_stats(data: &[f32]) -> (f32, f32, f32) {
    if data.is_empty() {
        return (0.0, 0.0, 0.0);
    }
    let mut min = f32::MAX;
    let mut max = f32::MIN;
    let mut sum_sq = 0.0f64;
    for &v in data {
        if v < min { min = v; }
        if v > max { max = v; }
        sum_sq += (v as f64) * (v as f64);
    }
    let rms = (sum_sq / data.len() as f64).sqrt() as f32;
    (min, max, rms)
}

fn generate_test_audio(seed: u64, amplitude: f32) -> Vec<f32> {
    let mut state = seed;
    let mut audio = Vec::with_capacity(48000);
    for _ in 0..48000 {
        state = state.wrapping_mul(1103515245).wrapping_add(12345);
        let random = ((state >> 16) & 0x7FFF) as f32 / 32767.0 * 2.0 - 1.0;
        audio.push(random * amplitude);
    }
    audio
}

/// Engine struct mimicking parakeet.rs structure
struct TestEngine {
    #[allow(dead_code)]
    core: Mutex<Core>,
    mel_request: Mutex<InferRequest>,
    encoder_request: Mutex<InferRequest>,
    decoder_request: Mutex<InferRequest>,
    joint_request: Mutex<InferRequest>,
    // Store models for recreation
    #[allow(dead_code)]
    mel_model: Mutex<CompiledModel>,
    encoder_model: Mutex<CompiledModel>,
    decoder_model: Mutex<CompiledModel>,
    joint_model: Mutex<CompiledModel>,
}

impl TestEngine {
    fn new(model_dir: &Path) -> Result<Self, String> {
        let mut core = Core::new().map_err(|e| format!("Core::new: {:?}", e))?;

        // Load all 4 models exactly like parakeet.rs
        let mut mel_model = Self::load_model(&mut core, model_dir, "parakeet_melspectogram")?;
        let mel_request = mel_model.create_infer_request()
            .map_err(|e| format!("mel create_infer_request: {:?}", e))?;

        let mut encoder_model = Self::load_model(&mut core, model_dir, "parakeet_encoder")?;
        let encoder_request = encoder_model.create_infer_request()
            .map_err(|e| format!("encoder create_infer_request: {:?}", e))?;

        let mut decoder_model = Self::load_model(&mut core, model_dir, "parakeet_decoder")?;
        let decoder_request = decoder_model.create_infer_request()
            .map_err(|e| format!("decoder create_infer_request: {:?}", e))?;

        let mut joint_model = Self::load_model(&mut core, model_dir, "parakeet_joint")?;
        let joint_request = joint_model.create_infer_request()
            .map_err(|e| format!("joint create_infer_request: {:?}", e))?;

        Ok(Self {
            core: Mutex::new(core),
            mel_request: Mutex::new(mel_request),
            encoder_request: Mutex::new(encoder_request),
            decoder_request: Mutex::new(decoder_request),
            joint_request: Mutex::new(joint_request),
            mel_model: Mutex::new(mel_model),
            encoder_model: Mutex::new(encoder_model),
            decoder_model: Mutex::new(decoder_model),
            joint_model: Mutex::new(joint_model),
        })
    }

    fn load_model(core: &mut Core, model_dir: &Path, name: &str) -> Result<CompiledModel, String> {
        let xml_path = model_dir.join(format!("{}.xml", name));
        let model = core.read_model_from_file(xml_path.to_str().unwrap(), "")
            .map_err(|e| format!("read_model {}: {:?}", name, e))?;
        core.compile_model(&model, DeviceType::CPU)
            .map_err(|e| format!("compile_model {}: {:?}", name, e))
    }

    /// Reset all requests by creating new ones (like reset_all_requests in parakeet.rs)
    fn reset_all_requests(&self) -> Result<(), String> {
        // Encoder
        {
            let mut model = self.encoder_model.lock().unwrap();
            let new_request = model.create_infer_request()
                .map_err(|e| format!("encoder create_infer_request: {:?}", e))?;
            let mut request = self.encoder_request.lock().unwrap();
            *request = new_request;
        }
        // Decoder
        {
            let mut model = self.decoder_model.lock().unwrap();
            let new_request = model.create_infer_request()
                .map_err(|e| format!("decoder create_infer_request: {:?}", e))?;
            let mut request = self.decoder_request.lock().unwrap();
            *request = new_request;
        }
        // Joint
        {
            let mut model = self.joint_model.lock().unwrap();
            let new_request = model.create_infer_request()
                .map_err(|e| format!("joint create_infer_request: {:?}", e))?;
            let mut request = self.joint_request.lock().unwrap();
            *request = new_request;
        }
        Ok(())
    }

    fn run_mel(&self, audio: &[f32]) -> Result<Vec<f32>, String> {
        let mut request = self.mel_request.lock().unwrap();

        let actual_len = audio.len().min(MAX_AUDIO_SAMPLES);
        let mut padded = vec![0.0f32; MAX_AUDIO_SAMPLES];
        padded[..actual_len].copy_from_slice(&audio[..actual_len]);

        {
            let mut input = request.get_tensor("input_signals")
                .map_err(|e| format!("mel get input: {:?}", e))?;
            let data = input.get_data_mut::<f32>()
                .map_err(|e| format!("mel input data: {:?}", e))?;
            data.fill(0.0);
            data[..MAX_AUDIO_SAMPLES].copy_from_slice(&padded);
        }

        {
            let mut length = request.get_tensor("input_length")
                .map_err(|e| format!("mel get length: {:?}", e))?;
            length.get_data_mut::<i64>()
                .map_err(|e| format!("mel length data: {:?}", e))?[0] = actual_len as i64;
        }

        request.infer().map_err(|e| format!("mel infer: {:?}", e))?;

        let output = request.get_output_tensor_by_index(0)
            .map_err(|e| format!("mel get output: {:?}", e))?;
        let data = output.get_data::<f32>()
            .map_err(|e| format!("mel output data: {:?}", e))?;

        Ok(data.to_vec())
    }

    fn run_encoder(&self, mel: &[f32], valid_frames: usize) -> Result<(Vec<f32>, usize, f32), String> {
        let mut request = self.encoder_request.lock().unwrap();

        // Copy mel features with proper stride
        let frames_to_copy = valid_frames.min(MAX_MEL_FRAMES);
        let mut padded_mel = vec![0.0f32; MEL_FEATURES * MAX_MEL_FRAMES];
        for f in 0..MEL_FEATURES {
            for t in 0..frames_to_copy {
                let src_idx = f * MAX_MEL_FRAMES + t;
                if src_idx < mel.len() {
                    padded_mel[f * MAX_MEL_FRAMES + t] = mel[src_idx];
                }
            }
        }

        {
            let mut input = request.get_tensor("melspectogram")
                .map_err(|e| format!("encoder get input: {:?}", e))?;
            let data = input.get_data_mut::<f32>()
                .map_err(|e| format!("encoder input data: {:?}", e))?;
            data.fill(0.0);
            data[..padded_mel.len()].copy_from_slice(&padded_mel);
        }

        {
            let mut length = request.get_tensor("melspectogram_length")
                .map_err(|e| format!("encoder get length: {:?}", e))?;
            length.get_data_mut::<i32>()
                .map_err(|e| format!("encoder length data: {:?}", e))?[0] = frames_to_copy as i32;
        }

        request.infer().map_err(|e| format!("encoder infer: {:?}", e))?;

        let output = request.get_tensor("encoder_output")
            .map_err(|e| format!("encoder get output: {:?}", e))?;
        let output_data = output.get_data::<f32>()
            .map_err(|e| format!("encoder output data: {:?}", e))?;

        let length_out = request.get_tensor("encoder_output_length")
            .map_err(|e| format!("encoder get length out: {:?}", e))?;
        let valid_time = length_out.get_data::<i64>()
            .map_err(|e| format!("encoder length out data: {:?}", e))?[0] as usize;

        let (_, _, rms) = compute_stats(output_data);
        Ok((output_data.to_vec(), valid_time, rms))
    }

    /// Run decoder step (simplified - just one iteration)
    fn run_decoder_step(&self, target: i64, h_in: &[f32], c_in: &[f32]) -> Result<(Vec<f32>, Vec<f32>, Vec<f32>), String> {
        let mut request = self.decoder_request.lock().unwrap();

        {
            let mut target_tensor = request.get_tensor("targets")
                .map_err(|e| format!("decoder get targets: {:?}", e))?;
            target_tensor.get_data_mut::<i64>()
                .map_err(|e| format!("decoder targets data: {:?}", e))?[0] = target;
        }

        {
            let mut h = request.get_tensor("h_in")
                .map_err(|e| format!("decoder get h_in: {:?}", e))?;
            let data = h.get_data_mut::<f32>()
                .map_err(|e| format!("decoder h_in data: {:?}", e))?;
            data.fill(0.0);
            data[..h_in.len()].copy_from_slice(h_in);
        }

        {
            let mut c = request.get_tensor("c_in")
                .map_err(|e| format!("decoder get c_in: {:?}", e))?;
            let data = c.get_data_mut::<f32>()
                .map_err(|e| format!("decoder c_in data: {:?}", e))?;
            data.fill(0.0);
            data[..c_in.len()].copy_from_slice(c_in);
        }

        request.infer().map_err(|e| format!("decoder infer: {:?}", e))?;

        let dec_out = request.get_tensor("decoder_output")
            .map_err(|e| format!("decoder get output: {:?}", e))?;
        let h_out = request.get_tensor("h_out")
            .map_err(|e| format!("decoder get h_out: {:?}", e))?;
        let c_out = request.get_tensor("c_out")
            .map_err(|e| format!("decoder get c_out: {:?}", e))?;

        Ok((
            dec_out.get_data::<f32>().map_err(|e| format!("decoder out data: {:?}", e))?.to_vec(),
            h_out.get_data::<f32>().map_err(|e| format!("h_out data: {:?}", e))?.to_vec(),
            c_out.get_data::<f32>().map_err(|e| format!("c_out data: {:?}", e))?.to_vec(),
        ))
    }

    /// Run joint step
    fn run_joint_step(&self, encoder_frame: &[f32], decoder_out: &[f32]) -> Result<Vec<f32>, String> {
        let mut request = self.joint_request.lock().unwrap();

        {
            let mut enc = request.get_tensor("encoder_outputs")
                .map_err(|e| format!("joint get enc: {:?}", e))?;
            let data = enc.get_data_mut::<f32>()
                .map_err(|e| format!("joint enc data: {:?}", e))?;
            data.fill(0.0);
            data[..encoder_frame.len()].copy_from_slice(encoder_frame);
        }

        {
            let mut dec = request.get_tensor("decoder_outputs")
                .map_err(|e| format!("joint get dec: {:?}", e))?;
            let data = dec.get_data_mut::<f32>()
                .map_err(|e| format!("joint dec data: {:?}", e))?;
            data.fill(0.0);
            data[..decoder_out.len()].copy_from_slice(decoder_out);
        }

        request.infer().map_err(|e| format!("joint infer: {:?}", e))?;

        let output = request.get_output_tensor()
            .map_err(|e| format!("joint get output: {:?}", e))?;
        let logits = output.get_data::<f32>()
            .map_err(|e| format!("joint output data: {:?}", e))?;

        Ok(logits.to_vec())
    }

    /// Run full pipeline like parakeet.rs (mel -> encoder -> decoder/joint loop)
    fn run_full_inference(&self, audio: &[f32]) -> Result<f32, String> {
        // Step 1: Mel
        let mel = self.run_mel(audio)?;
        let valid_frames = (audio.len() / HOP_LENGTH).min(MAX_MEL_FRAMES);

        // Step 2: Encoder
        let (encoder_out, valid_time, encoder_rms) = self.run_encoder(&mel, valid_frames)?;

        // Step 3: A few decoder/joint iterations (simplified)
        let h_state = vec![0.0f32; DECODER_NUM_LAYERS * DECODER_HIDDEN_DIM];
        let c_state = vec![0.0f32; DECODER_NUM_LAYERS * DECODER_HIDDEN_DIM];
        let last_token: i64 = 8192; // BLANK

        // Just run a few iterations to exercise the pipeline
        for t in 0..valid_time.min(5) {
            // Extract encoder frame at time t
            let mut encoder_frame = vec![0.0f32; ENCODER_OUTPUT_DIM];
            for i in 0..ENCODER_OUTPUT_DIM {
                encoder_frame[i] = encoder_out[i * MAX_ENCODER_TIME + t];
            }

            // Decoder
            let (dec_out, _, _) = self.run_decoder_step(last_token, &h_state, &c_state)?;

            // Joint
            let _logits = self.run_joint_step(&encoder_frame, &dec_out)?;
        }

        Ok(encoder_rms)
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Full Pipeline State Test (mimicking parakeet.rs)");
    println!("=================================================\n");

    std::env::set_var("OPENVINO_LIB_PATH", "/usr/local/lib");

    let model_dir = get_model_dir();
    let model_path = Path::new(&model_dir);

    println!("Loading all 4 models...");
    let engine = TestEngine::new(model_path)?;
    println!("All models loaded.\n");

    let audio = generate_test_audio(42, 0.1);
    println!("Test audio: {} samples\n", audio.len());

    // ========================================
    // TEST 1: Full pipeline without reset
    // ========================================
    println!("TEST 1: Full pipeline, no reset between inferences");
    println!("---------------------------------------------------");
    {
        let mut rms_values = Vec::new();
        for i in 0..5 {
            let rms = engine.run_full_inference(&audio)?;
            rms_values.push(rms);
            println!("  Inference {}: Encoder RMS = {:.6}", i + 1, rms);
        }

        let drift = rms_values.iter().cloned().fold(f32::MIN, f32::max)
            - rms_values.iter().cloned().fold(f32::MAX, f32::min);
        println!("\n  RMS drift: {:.6}", drift);
        if drift > 0.001 {
            println!("  ❌ FAIL: State accumulates in full pipeline");
        } else {
            println!("  ✅ PASS: No state accumulation");
        }
    }

    // ========================================
    // TEST 2: Full pipeline WITH reset_all_requests
    // ========================================
    println!("\nTEST 2: Full pipeline with reset_all_requests() between inferences");
    println!("-------------------------------------------------------------------");
    {
        let mut rms_values = Vec::new();
        for i in 0..5 {
            engine.reset_all_requests()?;
            let rms = engine.run_full_inference(&audio)?;
            rms_values.push(rms);
            println!("  Inference {} (after reset): Encoder RMS = {:.6}", i + 1, rms);
        }

        let drift = rms_values.iter().cloned().fold(f32::MIN, f32::max)
            - rms_values.iter().cloned().fold(f32::MAX, f32::min);
        println!("\n  RMS drift: {:.6}", drift);
        if drift > 0.001 {
            println!("  ❌ FAIL: reset_all_requests doesn't fix it");
        } else {
            println!("  ✅ PASS: reset_all_requests works");
        }
    }

    // ========================================
    // TEST 3: Encoder only (no decoder/joint)
    // ========================================
    println!("\nTEST 3: Encoder only (no decoder/joint)");
    println!("----------------------------------------");
    {
        let mel = engine.run_mel(&audio)?;
        let valid_frames = (audio.len() / HOP_LENGTH).min(MAX_MEL_FRAMES);

        let mut rms_values = Vec::new();
        for i in 0..5 {
            let (_, _, rms) = engine.run_encoder(&mel, valid_frames)?;
            rms_values.push(rms);
            println!("  Encoder only {}: RMS = {:.6}", i + 1, rms);
        }

        let drift = rms_values.iter().cloned().fold(f32::MIN, f32::max)
            - rms_values.iter().cloned().fold(f32::MAX, f32::min);
        println!("\n  RMS drift: {:.6}", drift);
        if drift > 0.001 {
            println!("  ❌ FAIL: Encoder alone accumulates state");
        } else {
            println!("  ✅ PASS: Encoder alone is stable");
        }
    }

    // ========================================
    // TEST 4: Check if decoder/joint affect encoder
    // ========================================
    println!("\nTEST 4: Run decoder/joint, then check encoder");
    println!("----------------------------------------------");
    {
        let mel = engine.run_mel(&audio)?;
        let valid_frames = (audio.len() / HOP_LENGTH).min(MAX_MEL_FRAMES);

        // First encoder run
        let (enc_out1, valid_time, rms1) = engine.run_encoder(&mel, valid_frames)?;
        println!("  Encoder before decoder/joint: RMS = {:.6}", rms1);

        // Run decoder/joint many times
        let h_state = vec![0.0f32; DECODER_NUM_LAYERS * DECODER_HIDDEN_DIM];
        let c_state = vec![0.0f32; DECODER_NUM_LAYERS * DECODER_HIDDEN_DIM];
        for t in 0..valid_time.min(20) {
            let mut encoder_frame = vec![0.0f32; ENCODER_OUTPUT_DIM];
            for i in 0..ENCODER_OUTPUT_DIM {
                encoder_frame[i] = enc_out1[i * MAX_ENCODER_TIME + t];
            }
            let (dec_out, _, _) = engine.run_decoder_step(8192, &h_state, &c_state)?;
            let _ = engine.run_joint_step(&encoder_frame, &dec_out)?;
        }
        println!("  Ran {} decoder/joint iterations", valid_time.min(20));

        // Second encoder run
        let (_, _, rms2) = engine.run_encoder(&mel, valid_frames)?;
        println!("  Encoder after decoder/joint: RMS = {:.6}", rms2);

        let drift = (rms1 - rms2).abs();
        println!("\n  RMS drift: {:.6}", drift);
        if drift > 0.001 {
            println!("  ❌ FAIL: Decoder/joint affect encoder state");
        } else {
            println!("  ✅ PASS: Decoder/joint don't affect encoder");
        }
    }

    println!("\n=================================================");
    println!("ANALYSIS COMPLETE");
    println!("=================================================");

    Ok(())
}
