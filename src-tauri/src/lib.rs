mod audio;
mod commands;
mod engine;
mod error;
mod export;
mod storage;

use commands::{AudioState, EngineState};
use parking_lot::Mutex;
use std::path::PathBuf;
use std::fs::File;
use tracing::{info, warn};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// Initialize OpenVINO library path for runtime linking
fn init_openvino() -> bool {
    // Check common OpenVINO library paths on macOS
    let openvino_paths = [
        "/usr/local/lib",  // Homebrew symlinks
        "/usr/local/Cellar/openvino/2025.4.1_3/lib",  // Homebrew direct
        "/opt/intel/openvino/runtime/lib",  // Intel installer
    ];

    for path in openvino_paths {
        let lib_path = format!("{}/libopenvino_c.dylib", path);
        if std::path::Path::new(&lib_path).exists() {
            // Set multiple environment variables to help library loading
            std::env::set_var("OPENVINO_LIB_PATH", path);
            std::env::set_var("OV_LIB_PATH", path);
            std::env::set_var("INTEL_OPENVINO_DIR", path);
            // Also set DYLD_LIBRARY_PATH for runtime loading on macOS
            if let Ok(existing) = std::env::var("DYLD_LIBRARY_PATH") {
                std::env::set_var("DYLD_LIBRARY_PATH", format!("{}:{}", path, existing));
            } else {
                std::env::set_var("DYLD_LIBRARY_PATH", path);
            }
            info!("Found OpenVINO library at {}", path);
            return true;
        }
    }

    warn!("OpenVINO library not found in standard paths");
    false
}

/// Get the model directory path
fn get_model_path() -> Option<PathBuf> {
    // Try relative to executable first (for development)
    if let Ok(exe_path) = std::env::current_exe() {
        // In development: src-tauri/target/debug/wakascribe
        // Model is at project_root/model (4 levels up)
        let mut path = exe_path.clone();

        // Go up from src-tauri/target/debug/wakascribe to project root
        for _ in 0..4 {
            path.pop();
        }
        path.push("model");

        if path.exists() {
            return Some(path);
        }

        // Try from the bundle (macOS .app)
        let mut bundle_path = exe_path;
        bundle_path.pop(); // Remove executable
        bundle_path.pop(); // Remove MacOS
        bundle_path.push("Resources");
        bundle_path.push("model");

        if bundle_path.exists() {
            return Some(bundle_path);
        }
    }

    // Try current directory
    let current = PathBuf::from("model");
    if current.exists() {
        return Some(current);
    }

    None
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize logging to both console and file
    let log_file = File::create("/tmp/wakascribe.log").expect("Failed to create log file");

    tracing_subscriber::registry()
        .with(fmt::layer()) // Console output
        .with(
            fmt::layer()
                .with_ansi(false)
                .with_writer(std::sync::Mutex::new(log_file))
        ) // File output
        .with(EnvFilter::from_default_env().add_directive("wakascribe=debug".parse().unwrap()))
        .init();

    info!("Starting WakaScribe...");

    // Initialize database
    if let Err(e) = storage::init_database() {
        eprintln!("Failed to initialize database: {}", e);
    }

    // Initialize OpenVINO
    let openvino_ok = init_openvino();

    // Initialize engine and load model
    let mut engine = engine::ParakeetEngine::new();

    if openvino_ok {
        if let Some(model_path) = get_model_path() {
            info!("Found model at {:?}", model_path);
            if let Err(e) = engine.load_model(&model_path) {
                warn!("Failed to load model: {}. Using mock transcription.", e);
            }
        } else {
            warn!("Model not found. Using mock transcription.");
        }
    } else {
        warn!("OpenVINO not available. Using mock transcription.");
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .manage(AudioState(audio::AudioCapture::new()))
        .manage(EngineState(Mutex::new(engine)))
        .invoke_handler(tauri::generate_handler![
            // Audio commands
            commands::list_audio_devices,
            commands::start_recording,
            commands::stop_recording,
            commands::pause_recording,
            commands::resume_recording,
            commands::get_audio_level,
            // Transcription commands
            commands::transcribe_file,
            commands::get_transcription,
            // History commands
            commands::list_transcriptions,
            commands::delete_transcription,
            commands::update_transcription_text,
            // Settings commands
            commands::get_settings,
            commands::update_settings,
            // Export commands
            commands::export_to_txt,
            commands::export_to_docx,
            commands::copy_to_clipboard,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
