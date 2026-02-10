# TODO: Nettoyage moteurs - Supprimer ONNX Runtime

## Contexte

Revue de code du 6 février 2026. La duplication massive (~970 lignes) est entre `parakeet.rs` (OpenVINO) et `onnxruntime.rs`. CoreML étant un sidecar Swift, il n'a quasiment aucun code commun avec OpenVINO. On ne garde que OpenVINO + CoreML.

---

## ~~Phase 1 : Supprimer ONNX Runtime (~870 lignes)~~ DONE

- [x] Supprimer `src-tauri/src/engine/onnxruntime.rs`
- [x] Retirer `pub mod onnxruntime;` de `engine/mod.rs`
- [x] Retirer `pub use onnxruntime::OnnxRuntimeEngine;` de `engine/mod.rs`
- [x] Supprimer le variant `OnnxRuntime` de l'enum `EngineBackend` (mod.rs)
- [x] Retirer les branches `OnnxRuntime` dans `DynamicEngine::new()` et `switch_backend()`
- [x] Retirer les dépendances `ort` / onnxruntime de `Cargo.toml`
- [x] Supprimer le dossier `model/onnxruntime/`
- [x] Mettre à jour les Settings frontend (retirer l'option onnxruntime dans EngineSettings.tsx)

## ~~Phase 2 : Supprimer la duplication `transcribe()` / `mock_transcribe()`~~ DONE

- [x] Supprimer `ParakeetEngine::transcribe()` — c'est `DynamicEngine` qui wrappe en `Transcription`
- [x] Supprimer `ParakeetEngine::mock_transcribe()` — doublon de `DynamicEngine::mock_transcribe()`
- [x] Vérifier qu'aucun code n'appelle `ParakeetEngine::transcribe()` directement (tout passe par `DynamicEngine`)

## ~~Phase 3 : Déplacer `TranscriptionLanguage`~~ DONE

- [x] Déplacer `TranscriptionLanguage` (enum + impls `token_id()`, `display_name()`, `code()`) vers `engine/mod.rs`

## ~~Phase 4 : Vérifier modules potentiellement inutiles~~ DONE

- [x] `engine/mel.rs` — supprimé (mel intégré directement dans parakeet.rs)
- [x] `engine/merger.rs` — supprimé (non utilisé)
- [x] `engine/decoder.rs` — conservé (utilisé activement), `load_txt()` retiré

## ~~Phase 5 : Hygiène~~ DONE

- [x] Retirer chemin hardcodé dans `coreml.rs` (utilise maintenant `current_exe()` + chemins relatifs)
- [x] Retirer chemin hardcodé dans `commands/test_transcription.rs` (utilise maintenant des chemins relatifs)
- [x] `ClipboardExt` dans `commands/export.rs` — en fait utilisé (trait method `clipboard()`), conservé

---

## ~~Déduplication code (11 items)~~ DONE — 10 février 2026

### ~~Backend Rust (5 items)~~ DONE

- [x] R1: `prepare_audio()` dans `audio/mod.rs` — resample + normalize en un appel (3 sites: `commands/transcription.rs`, `commands/test_transcription.rs`)
- [x] R2: `get_transcription_or_error()` dans `storage/queries.rs` (3 sites: `commands/export.rs` x2, `commands/transcription.rs`)
- [x] R3: Unifié `calculate_rms()` dans `processor.rs` avec précision f64 — supprimé `compute_rms()` de `vad.rs`, `test_transcription.rs`, et calcul inline de `capture.rs`
- [x] R4: `format_duration()` / `format_timestamp()` dans `export/mod.rs` — supprimé de `txt.rs` et `docx.rs`
- [x] R5: `row_to_transcription()` dans `storage/queries.rs` — utilisé par `get_transcription()` et `list_transcriptions()`

### ~~Frontend React (6 items)~~ DONE

- [x] F1: `src/lib/formatters.ts` — `formatTime(ms)` + `formatDuration(ms)` (5 composants: ProgressBar, RecordingControls, SegmentList, Recorder, TranscriptionCard)
- [x] F2: `src/lib/config.ts` — `getDecodingConfig(settings)` (2 hooks: useRecording, useTranscription)
- [x] F3: `src/components/ui/CopyButton.tsx` (2 composants: Recorder, FileTranscribe)
- [x] F4: `src/components/ui/PanelHeader.tsx` avec props `title`, `onClose`, `subtitle?`, `actions?` (3 composants: Settings, History, Editor)
- [x] F5: `src/components/ui/RangeSlider.tsx` avec `formatValue?`, `minLabel?`, `maxLabel?` (3 sliders dans TranscriptionSettings)
- [x] F6: 6 classes Tailwind `@layer components` dans `index.css` : `.btn-secondary`, `.btn-primary`, `.panel`, `.btn-toggle`, `.btn-toggle-active`, `.menu-item` (8 composants mis à jour)
