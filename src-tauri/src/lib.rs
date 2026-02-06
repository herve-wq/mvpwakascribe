mod audio;
mod commands;
pub mod engine;
mod error;
mod export;
mod storage;

use commands::{AudioState, EngineState, ModelPathState};
use parking_lot::Mutex;
use std::path::PathBuf;
use std::fs::File;
use tracing::{info, warn};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// Initialize OpenVINO library path for runtime linking
fn init_openvino() -> bool {
    #[cfg(target_os = "macos")]
    let (paths, lib_name, ld_var) = (
        vec![
            "/usr/local/lib".to_string(),
            "/usr/local/Cellar/openvino/2025.4.1_3/lib".to_string(),
            "/opt/intel/openvino/runtime/lib".to_string(),
        ],
        "libopenvino_c.dylib",
        "DYLD_LIBRARY_PATH",
    );

    #[cfg(target_os = "windows")]
    let (paths, lib_name, ld_var) = (
        {
            let mut p = vec![
                r"C:\Program Files (x86)\Intel\openvino\runtime\bin\intel64\Release".to_string(),
                r"C:\Program Files (x86)\Intel\openvino\runtime\bin\intel64\Debug".to_string(),
                r"C:\Program Files\Intel\openvino\runtime\bin\intel64\Release".to_string(),
            ];
            // Also check next to the executable
            if let Ok(exe) = std::env::current_exe() {
                if let Some(dir) = exe.parent() {
                    p.insert(0, dir.to_string_lossy().to_string());
                }
            }
            p
        },
        "openvino_c.dll",
        "PATH",
    );

    #[cfg(target_os = "linux")]
    let (paths, lib_name, ld_var) = (
        vec![
            "/usr/lib/x86_64-linux-gnu".to_string(),
            "/usr/local/lib".to_string(),
            "/opt/intel/openvino/runtime/lib/intel64".to_string(),
        ],
        "libopenvino_c.so",
        "LD_LIBRARY_PATH",
    );

    for path in &paths {
        let lib_path = format!("{}/{}", path, lib_name);
        if std::path::Path::new(&lib_path).exists() {
            std::env::set_var("OPENVINO_LIB_PATH", path);
            std::env::set_var("OV_LIB_PATH", path);
            std::env::set_var("INTEL_OPENVINO_DIR", path);
            if let Ok(existing) = std::env::var(ld_var) {
                let sep = if cfg!(windows) { ";" } else { ":" };
                std::env::set_var(ld_var, format!("{}{}{}", path, sep, existing));
            } else {
                std::env::set_var(ld_var, path);
            }
            info!("Found OpenVINO library at {}", path);
            return true;
        }
    }

    warn!("OpenVINO library not found in standard paths");
    false
}

/// Cross-platform app data directory
fn app_data_dir() -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        std::env::var_os("HOME")
            .map(PathBuf::from)
            .map(|p| p.join("Library/Application Support"))
    }

    #[cfg(target_os = "windows")]
    {
        std::env::var_os("LOCALAPPDATA").map(PathBuf::from)
    }

    #[cfg(target_os = "linux")]
    {
        std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME")
                    .map(PathBuf::from)
                    .map(|p| p.join(".local/share"))
            })
    }
}

/// Get the base model directory path
fn get_model_base_path() -> Option<PathBuf> {
    // 1. App data directory (production + test builds)
    // macOS: ~/Library/Application Support/com.wakascribe.app/models/
    // Windows: %LOCALAPPDATA%/com.wakascribe.app/models/
    // Linux: ~/.local/share/com.wakascribe.app/models/
    if let Some(app_models) = app_data_dir().map(|p| p.join("com.wakascribe.app").join("models")) {
        if app_models.exists() {
            info!("Found models in app data directory: {:?}", app_models);
            return Some(app_models);
        }
    }

    // 2. Try relative to executable (for development)
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

        // 3. Try from the bundle (macOS .app)
        let mut bundle_path = exe_path;
        bundle_path.pop(); // Remove executable
        bundle_path.pop(); // Remove MacOS
        bundle_path.push("Resources");
        bundle_path.push("model");

        if bundle_path.exists() {
            return Some(bundle_path);
        }
    }

    // 4. Try current directory
    let current = PathBuf::from("model");
    if current.exists() {
        return Some(current);
    }

    None
}

/// Get model path for specific backend
fn get_model_path(backend: engine::EngineBackend) -> Option<PathBuf> {
    let base_path = get_model_base_path()?;
    let backend_path = base_path.join(backend.model_subdir());
    if backend_path.exists() {
        Some(backend_path)
    } else {
        None
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize logging to both console and file
    let log_path = std::env::temp_dir().join("wakascribe.log");
    let log_file = File::create(&log_path).expect("Failed to create log file");

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

    // Read saved engine backend preference from database
    let saved_backend = storage::with_db(|conn| storage::get_settings(conn))
        .ok()
        .map(|s| s.engine_backend)
        .unwrap_or_else(|| "openvino".to_string());
    info!("Saved engine backend preference: {}", saved_backend);

    // Initialize OpenVINO library path (needed if we want to use OpenVINO)
    let openvino_ok = init_openvino();

    // Get model base path
    let model_base_path = get_model_base_path().unwrap_or_else(|| PathBuf::from("model"));
    info!("Model base path: {:?}", model_base_path);

    // Determine which backend to use based on saved preference
    let (backend, engine_loaded) = match saved_backend.as_str() {
        "onnxruntime" => {
            info!("Loading saved preference: ONNX Runtime");
            try_load_backend(engine::EngineBackend::OnnxRuntime, openvino_ok)
        }
        #[cfg(target_os = "macos")]
        "coreml" => {
            info!("Loading saved preference: CoreML");
            try_load_backend(engine::EngineBackend::CoreML, openvino_ok)
        }
        _ => {
            // Default to OpenVINO
            info!("Loading saved preference: OpenVINO");
            try_load_backend(engine::EngineBackend::OpenVINO, openvino_ok)
        }
    };

    fn try_load_backend(preferred: engine::EngineBackend, openvino_ok: bool) -> (engine::DynamicEngine, bool) {
        // Try preferred backend first
        if let Some(model_path) = get_model_path(preferred) {
            // For OpenVINO, check if library is available
            if matches!(preferred, engine::EngineBackend::OpenVINO) && !openvino_ok {
                info!("OpenVINO library not available, trying fallback");
            } else {
                info!("Found {} model at {:?}", preferred.display_name(), model_path);
                let mut engine = engine::DynamicEngine::new(preferred);
                match engine.load_model(&model_path) {
                    Ok(_) => {
                        info!("{} engine loaded successfully", preferred.display_name());
                        return (engine, true);
                    }
                    Err(e) => {
                        warn!("Failed to load {} model: {}", preferred.display_name(), e);
                    }
                }
            }
        } else {
            info!("No {} model found", preferred.display_name());
        }

        // Fallback: try other backends
        let fallbacks = [
            engine::EngineBackend::OnnxRuntime,
            engine::EngineBackend::OpenVINO,
        ];

        for fallback in fallbacks {
            if fallback == preferred {
                continue;
            }
            if matches!(fallback, engine::EngineBackend::OpenVINO) && !openvino_ok {
                continue;
            }
            if let Some(model_path) = get_model_path(fallback) {
                info!("Trying fallback: {} from {:?}", fallback.display_name(), model_path);
                let mut engine = engine::DynamicEngine::new(fallback);
                match engine.load_model(&model_path) {
                    Ok(_) => {
                        info!("{} engine loaded successfully (fallback)", fallback.display_name());
                        return (engine, true);
                    }
                    Err(e) => {
                        warn!("Failed to load {} model: {}", fallback.display_name(), e);
                    }
                }
            }
        }

        // Nothing worked
        (engine::DynamicEngine::new(engine::EngineBackend::OpenVINO), false)
    }

    if !engine_loaded {
        warn!("No model loaded. Using mock transcription.");
    } else {
        info!("Using {} backend", backend.name());
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .manage(AudioState(audio::AudioCapture::new()))
        .manage(EngineState(Mutex::new(backend)))
        .manage(ModelPathState(model_base_path))
        .invoke_handler(tauri::generate_handler![
            // Audio commands
            commands::list_audio_devices,
            commands::start_recording,
            commands::stop_recording,
            commands::stop_recording_to_wav,
            commands::pause_recording,
            commands::resume_recording,
            commands::get_audio_level,
            // Transcription commands
            commands::transcribe_file,
            commands::get_transcription,
            // Engine commands
            commands::switch_engine_backend,
            commands::get_engine_backend,
            // History commands
            commands::list_transcriptions,
            commands::delete_transcription,
            commands::delete_all_transcriptions,
            commands::update_transcription_text,
            // Settings commands
            commands::get_settings,
            commands::update_settings,
            // Export commands
            commands::export_to_txt,
            commands::export_to_docx,
            commands::copy_to_clipboard,
            // Test commands - commenter pour désactiver
            commands::test_transcription,
            commands::check_test_audio,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
