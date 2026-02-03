//! Module de test de transcription avec fichier WAV de référence
//!
//! Ce module fournit une commande pour tester la transcription avec un fichier
//! audio prédéfini, permettant des tests reproductibles.
//!
//! Pour désactiver ce module:
//! 1. Commenter la ligne `pub mod test_transcription;` dans commands/mod.rs
//! 2. Commenter l'enregistrement de la commande dans lib.rs

use crate::audio::{load_audio_file, normalize_audio, resample_to_16k};
use crate::commands::EngineState;
use crate::engine::TranscriptionLanguage;
use crate::error::{AppError, Result};
use serde::Serialize;
use std::path::PathBuf;
use std::time::Instant;
use tauri::State;
use tracing::info;

/// Résultat du test de transcription avec métriques détaillées
#[derive(Debug, Serialize)]
pub struct TestTranscriptionResult {
    /// Texte transcrit
    pub text: String,
    /// Chemin du fichier audio utilisé
    pub audio_file: String,
    /// Durée de l'audio en ms
    pub audio_duration_ms: i64,
    /// Temps total de transcription en ms
    pub transcription_time_ms: u64,
    /// Ratio temps réel (1.0 = temps réel, 0.5 = 2x plus rapide)
    pub realtime_factor: f64,
    /// Métriques de diagnostic
    pub diagnostics: TestDiagnostics,
}

/// Métriques de diagnostic pour analyse
#[derive(Debug, Serialize)]
pub struct TestDiagnostics {
    /// RMS de l'audio d'entrée
    pub audio_rms: f32,
    /// Nombre de samples audio
    pub audio_samples: usize,
    /// Sample rate original
    pub original_sample_rate: u32,
    /// Nombre de tokens générés
    pub tokens_count: usize,
}

/// Trouve le fichier audio de test
fn find_test_audio() -> Result<PathBuf> {
    // Chercher dans plusieurs emplacements possibles
    let possible_paths = [
        // Développement: relatif à l'exécutable
        "../model/test_audio.wav",
        "../../model/test_audio.wav",
        "../../../model/test_audio.wav",
        "../../../../model/test_audio.wav",
        // Absolu pour développement
        "/Users/herve/dev/mvpparakeet/wakascribe/model/test_audio.wav",
        // Bundle macOS
        "../Resources/model/test_audio.wav",
    ];

    // Essayer relatif à l'exécutable
    if let Ok(exe_path) = std::env::current_exe() {
        let exe_dir = exe_path.parent().unwrap_or(&exe_path);
        for relative_path in &possible_paths {
            let full_path = exe_dir.join(relative_path);
            if full_path.exists() {
                return Ok(full_path);
            }
        }
    }

    // Essayer les chemins absolus
    for path in &possible_paths {
        let path = PathBuf::from(path);
        if path.exists() {
            return Ok(path);
        }
    }

    Err(AppError::NotFound(
        "Fichier test_audio.wav non trouvé. Placez un fichier test_audio.wav dans le dossier model/".to_string()
    ))
}

/// Calcule le RMS d'un signal audio
fn compute_rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum_sq: f64 = samples.iter().map(|&s| (s as f64) * (s as f64)).sum();
    (sum_sq / samples.len() as f64).sqrt() as f32
}

/// Commande Tauri pour tester la transcription avec le fichier de référence
///
/// Cette commande:
/// 1. Charge le fichier test_audio.wav depuis le dossier model/
/// 2. Le transcrit avec le moteur Parakeet
/// 3. Retourne le résultat avec des métriques détaillées
#[tauri::command]
pub fn test_transcription(
    engine_state: State<'_, EngineState>,
) -> Result<TestTranscriptionResult> {
    info!("=== TEST TRANSCRIPTION START ===");

    // Trouver le fichier de test
    let audio_path = find_test_audio()?;
    info!("Using test audio: {:?}", audio_path);

    // Charger l'audio
    let load_start = Instant::now();
    let (samples, sample_rate) = load_audio_file(&audio_path)?;
    let load_time = load_start.elapsed();
    info!("Audio loaded in {:?}: {} samples @ {}Hz", load_time, samples.len(), sample_rate);

    // Calculer les métriques audio
    let audio_rms = compute_rms(&samples);
    let audio_duration_ms = (samples.len() as f64 / sample_rate as f64 * 1000.0) as i64;
    info!("Audio: duration={}ms, rms={:.4}", audio_duration_ms, audio_rms);

    // Resampler à 16kHz
    let resample_start = Instant::now();
    let resampled = resample_to_16k(&samples, sample_rate)?;
    let resample_time = resample_start.elapsed();
    info!("Resampled in {:?}: {} -> {} samples", resample_time, samples.len(), resampled.len());

    // Normaliser le niveau audio
    let (normalized, gain) = normalize_audio(&resampled);
    info!("Audio normalized with gain {:.1}x", gain);

    // Transcrire (utilise Auto pour la détection automatique de langue)
    let transcribe_start = Instant::now();
    let engine = engine_state.0.lock();
    let transcription = engine.transcribe(
        &normalized,
        "test",
        Some("test_audio.wav".to_string()),
        TranscriptionLanguage::Auto,
    )?;
    let transcribe_time = transcribe_start.elapsed();

    let transcription_time_ms = transcribe_time.as_millis() as u64;
    let realtime_factor = transcription_time_ms as f64 / audio_duration_ms as f64;

    info!("Transcription completed in {:?}", transcribe_time);
    info!("Realtime factor: {:.2}x ({}ms audio / {}ms processing)",
          1.0 / realtime_factor, audio_duration_ms, transcription_time_ms);
    info!("Result: '{}'", transcription.raw_text);

    // Compter les tokens (approximation basée sur les espaces)
    let tokens_count = transcription.raw_text.split_whitespace().count();

    info!("=== TEST TRANSCRIPTION END ===");

    Ok(TestTranscriptionResult {
        text: transcription.raw_text,
        audio_file: audio_path.to_string_lossy().to_string(),
        audio_duration_ms,
        transcription_time_ms,
        realtime_factor,
        diagnostics: TestDiagnostics {
            audio_rms,
            audio_samples: samples.len(),
            original_sample_rate: sample_rate,
            tokens_count,
        },
    })
}

/// Commande pour vérifier si le fichier de test existe
#[tauri::command]
pub fn check_test_audio() -> Result<String> {
    let path = find_test_audio()?;
    Ok(path.to_string_lossy().to_string())
}
