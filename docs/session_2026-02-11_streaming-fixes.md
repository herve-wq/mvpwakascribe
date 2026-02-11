# Session 11 février 2026 — Corrections streaming OpenVINO

## Problème initial

Le streaming dictée OpenVINO perdait des bouts de phrase et produisait des hallucinations anglaises en fin de chunk (`Thank you. Thank you.`, `I'm not going to go.`). Diagnostiqué via 3 tests de dictée live puis confirmé par un outil de simulation.

## Corrections apportées

### 1. Filtrage du leftover silencieux en début de chunk

**Fichiers :** `audio/mod.rs`, `commands/transcription.rs`

Le leftover (audio résiduel après un cut VAD) était systématiquement préfixé au chunk suivant. S'il était silencieux, il déstabilisait les premières frames du décodeur → garbling, dérive de langue.

**Fix :** Calculer le RMS du leftover et le jeter s'il est en dessous d'un seuil.

### 2. Trim du silence trailing avant inférence

**Fichiers :** `audio/processor.rs`, `audio/mod.rs`, `commands/transcription.rs`

Le cut VAD coupe au point le plus silencieux → le chunk se termine dans une zone de silence. Le décodeur traite ces frames silencieuses et hallucine dans n'importe laquelle des 25 langues du modèle.

**Fix :** Nouvelle fonction `trim_trailing_silence()` qui scanne le chunk en arrière par fenêtres de 20ms, trouve la dernière fenêtre active, et garde 150ms de padding. Appliqué dans la boucle streaming et dans `stop_recording`.

### 3. Seuil relatif au lieu d'absolu

**Fichiers :** `audio/processor.rs`, `commands/transcription.rs`, `bin/test_streaming_sim.rs`

Le seuil absolu (0.01) classait de la vraie parole comme silence sur les enregistrements calmes (gain 18x → RMS parole ~0.008, sous le seuil).

**Fix :** Remplacé par `SILENCE_RATIO = 0.1` (10% du RMS du chunk). Le seuil s'adapte automatiquement au volume d'enregistrement. Appliqué au leftover discard et au trim trailing silence.

### 4. Binaire de test `test_streaming_sim`

**Fichiers :** `src/bin/test_streaming_sim.rs`, `Cargo.toml`, `lib.rs`

Outil CLI qui prend un WAV et compare single-shot vs streaming simulé sur le même audio. Élimine les biais de la dictée live (micro, voix, volume). Rendu possible par `pub mod audio`, `pub mod error`, `pub fn init_openvino()` dans `lib.rs`.

**Usage :** `cargo run --bin test_streaming_sim -- fichier.wav [french|english|auto]`

## Fichiers modifiés

| Fichier | Modification |
|---|---|
| `src-tauri/src/audio/processor.rs` | `trim_trailing_silence()` avec seuil relatif |
| `src-tauri/src/audio/mod.rs` | Réexport `calculate_rms`, `trim_trailing_silence` |
| `src-tauri/src/commands/transcription.rs` | `SILENCE_RATIO`, leftover discard relatif, trim dans streaming + stop_recording |
| `src-tauri/src/lib.rs` | `pub mod audio`, `pub mod error`, `pub fn init_openvino()` |
| `src-tauri/src/bin/test_streaming_sim.rs` | **Nouveau** — binaire de test streaming simulé |
| `src-tauri/Cargo.toml` | Ajout `[[bin]] test_streaming_sim` |

## Résultats validés

| Test | Avant | Après |
|---|---|---|
| Audio calme (21s, gain 18x) | Perte de phrases + hallucinations anglaises | Tout le contenu récupéré, 3 chunks propres |
| Audio normal (56s, gain 6.8x) | Fonctionnait déjà | Pas de régression, silence final toujours discardé |

## Analyse technique

Le modèle Parakeet TDT v3 supporte 25 langues. Quand le décodeur traite du silence, il n'a plus de signal phonétique et génère depuis son prior de modèle de langue — n'importe quelle langue peut sortir. Un filtre textuel par langue est impossible (25 langues). La solution est au niveau signal : ne pas nourrir le décodeur avec du silence inutile (trim trailing + discard leftover silencieux).

Le seuil relatif (10% du RMS du chunk) résout le problème de calibration : un seuil absolu ne peut pas fonctionner car le RMS brut de la parole dépend du volume d'enregistrement (micro, distance, gain).

## Prochaines étapes possibles

- Tester en dictée live avec les corrections
- Le leading filter (`filter_chunk_hallucinations`) pourrait bénéficier du même traitement (quelques artefacts en début de chunk : `100% 6.`, ponctuation parasite)
- Ajuster le timing des segments streaming (timestamps approximatifs)
