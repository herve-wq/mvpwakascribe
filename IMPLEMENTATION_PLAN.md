# WakaScribe - Plan d'Implémentation Parakeet

## Date: 2026-02-03

## Contexte

WakaScribe utilise le modèle Parakeet TDT v3 (NVIDIA NeMo) via OpenVINO pour la transcription audio. Plusieurs problèmes ont été identifiés et des optimisations sont nécessaires.

---

## Problèmes Identifiés

### Bug Critique: Accumulation Buffer Audio

**Symptôme**: Les samples audio s'accumulent entre les enregistrements successifs.

| Enregistrement | Durée réelle | Samples attendus | Samples réels | Ratio |
|----------------|--------------|------------------|---------------|-------|
| #1 | 7.9s | 126,400 | 126,400 | 1.0x ✅ |
| #2 | 8.6s | 137,600 | 276,480 | 2.0x ❌ |
| #3 | 6.5s | 104,000 | 314,880 | 3.0x ❌ |

**Fichier concerné**: `src-tauri/src/audio/capture.rs`

**Fix partiel appliqué**:
- Clear buffer avant création du nouveau stream
- Délai 50ms pour laisser les callbacks terminer
- Ordre des opérations corrigé dans Start/Stop handlers

**Hypothèses restantes**:
1. Buffer interne du device Bluetooth "Zone 301"
2. Race condition persistante dans cpal
3. Problème spécifique au device

**Test à faire**: Essayer avec le micro intégré MacBook Pro (48kHz) au lieu du Bluetooth (16kHz)

---

### Problème: Hallucinations sur signal faible

**Symptôme**: Quand le RMS audio est < 0.01, le modèle génère du texte incohérent (souvent en anglais).

**Solution implémentée**: Normalisation audio avec target RMS = 0.05

**Fichier**: `src-tauri/src/audio/processor.rs`

```rust
const TARGET_RMS: f32 = 0.05;
const MIN_RMS_THRESHOLD: f32 = 0.001;

pub fn normalize_audio(samples: &[f32]) -> (Vec<f32>, f32)
```

---

### Problème: Troncature audio > 15s

**Symptôme**: Audio tronqué à 240,000 samples (15s @ 16kHz)

**Fichier**: `src-tauri/src/engine/parakeet.rs`

```rust
const MAX_AUDIO_SAMPLES: usize = 240000;
```

**Solution requise**: Implémenter chunking avec overlap

---

## Requis Parakeet/NeMo

### Preprocessing Audio

| Paramètre | Valeur requise | État actuel |
|-----------|----------------|-------------|
| Sample Rate | 16,000 Hz | ✅ Resampling implémenté |
| Canaux | Mono | ⚠️ Fichier OK, Micro à vérifier |
| Normalisation | Peak -3dB (~0.708) | ⚠️ RMS 0.05 implémenté |
| Anti-clipping | Soft limiting | ⚠️ Basique |

### Chunking

| Paramètre | Valeur recommandée |
|-----------|-------------------|
| Chunk size | 10-15 secondes |
| Overlap | 1-2 secondes |
| Fusion | Déduplication par timestamps |

### Décodage

| Paramètre | Valeur actuelle | Valeur recommandée |
|-----------|-----------------|-------------------|
| Beam Width | 1 (Greedy) | 5-10 |
| Temperature | Non implémenté | 0.5-0.7 |

---

## Architecture Fichiers

```
src-tauri/src/
├── audio/
│   ├── capture.rs      # Capture micro (BUG: accumulation)
│   ├── processor.rs    # Resampling + Normalisation
│   ├── mod.rs
│   ├── chunker.rs      # À CRÉER: Découpage audio
│   └── quality.rs      # À CRÉER: Métriques qualité
├── engine/
│   ├── parakeet.rs     # Inférence OpenVINO
│   ├── decoder.rs      # Décodage TDT (Greedy)
│   ├── mel.rs          # Mel spectrogram
│   ├── merger.rs       # À CRÉER: Fusion transcriptions
│   └── config.rs       # À CRÉER: Configuration
└── commands/
    ├── transcription.rs
    └── test_transcription.rs
```

---

## Plan d'Implémentation

### Phase 0: Bug Critique (P0) ✅

#### 0.1 Diagnostic approfondi
- [x] Ajouter logs dans callback audio (samples écrits, is_recording state)
- [ ] Tester avec micro intégré MacBook Pro
- [ ] Comparer comportement Bluetooth vs Built-in

#### 0.2 Fix définitif
- [x] Identifier source exacte du cumul → **Race condition TOCTOU dans callbacks**
- [x] Implémenter synchronisation robuste → **Compteur de génération (recording_generation)**

**Solution implémentée**: Chaque enregistrement reçoit un ID unique (génération). Les callbacks vérifient que leur génération correspond à la génération courante avant d'écrire dans le buffer. Cela empêche les callbacks "stale" d'écrire des samples dans le mauvais enregistrement.

### Phase 1: Preprocessing Audio (P1) ✅

#### 1.1 Conversion Mono robuste
- [x] Vérifier si micro capture en stéréo → Micro capture déjà en mono (1 channel)
- [x] Conversion mono dans `processor.rs` pour fichiers stéréo (déjà implémenté)

#### 1.2 Audio Chunking ✅
- [x] Créer `src/audio/chunker.rs`
- [x] Paramètres: CHUNK_SIZE=10s, OVERLAP=2s
- [x] Découper audio long en segments
- [x] Tests unitaires (2 tests passent)

#### 1.3 Fusion transcriptions ✅
- [x] Créer `src/engine/merger.rs`
- [x] Déduplication basée sur proportion overlap/durée
- [x] Nettoyage ponctuation et espaces
- [x] Tests unitaires (3 tests passent)

#### 1.4 Intégration dans parakeet.rs ✅
- [x] `run_inference()` détecte audio > 15s automatiquement
- [x] `run_chunked_inference()` traite par chunks de 10s avec 2s overlap
- [x] `run_single_inference()` pour chunks individuels

### Phase 2: Normalisation Audio (P2)

#### 2.1 Normalisation Peak -3dB
- [ ] Modifier `normalize_audio()` pour target peak au lieu de RMS
- [ ] Target: 0.708 (-3dB)

#### 2.2 Anti-clipping amélioré
- [ ] Détection pre-clip (samples > 0.95)
- [ ] Soft limiting progressif
- [ ] Log warnings si clipping détecté

#### 2.3 Détection qualité audio
- [ ] Créer `src/audio/quality.rs`
- [ ] Métriques: SNR, silence ratio, clipping ratio
- [ ] Score qualité 0-100

### Phase 3: Décodage Avancé (P3)

#### 3.1 Beam Search
- [ ] Modifier `decoder.rs` pour supporter beam_width > 1
- [ ] Paramètre configurable (défaut: 5)

#### 3.2 Temperature
- [ ] Ajouter paramètre temperature dans inférence
- [ ] Application: logits / temperature avant softmax

#### 3.3 Configuration exposée
- [ ] Créer `src/engine/config.rs`
- [ ] Struct `TranscriptionConfig` avec tous les paramètres

### Phase 4: UI/UX (P4)

#### 4.1 Paramètres avancés
- [ ] Section "Avancé" dans Settings
- [ ] Sliders: beam_width, temperature
- [ ] Toggle: chunking activé/désactivé

#### 4.2 Indicateurs qualité
- [ ] Afficher score qualité avant transcription
- [ ] Warnings visuels (RMS faible, clipping)

---

## Configuration Cible

```rust
struct TranscriptionConfig {
    // Preprocessing
    normalize: bool,           // default: true
    target_db: f32,            // default: -3.0

    // Chunking
    enable_chunking: bool,     // default: true
    chunk_seconds: f32,        // default: 10.0
    overlap_seconds: f32,      // default: 2.0

    // Decoding
    beam_width: u32,           // default: 5
    temperature: f32,          // default: 0.7

    // Quality
    min_audio_rms: f32,        // default: 0.01
    reject_low_quality: bool,  // default: false
}
```

---

## Tests de Validation

| Test | Critère de succès |
|------|-------------------|
| 3 enregistrements 5s consécutifs | Samples = durée × 16000 (±5%) |
| Audio 60s | Transcription complète, pas de troncature |
| Audio très faible (RMS < 0.01) | Warning affiché, pas d'hallucination |
| Beam=1 vs Beam=5 | Comparer WER sur même audio |
| Fichier test WAV 3x | Résultats identiques |

---

## Commandes Utiles

```bash
# Lancer l'application
cd wakascribe && npm run tauri:dev

# Voir les logs
tail -f /tmp/wakascribe.log | grep -E "DIAG|Encoder|Decoded|Buffer|Normalizing"

# Vérifier config audio système
system_profiler SPAudioDataType

# Build release
npm run tauri:build
```

---

## Références

- [NVIDIA NeMo Parakeet](https://docs.nvidia.com/nemo-framework/user-guide/latest/nemotoolkit/asr/models.html)
- [OpenVINO Rust Bindings](https://github.com/intel/openvino-rs)
- [cpal Audio Library](https://github.com/RustAudio/cpal)

---

## Historique des Modifications

### 2026-02-03
- Identifié bug accumulation buffer audio
- Implémenté normalisation audio (RMS target 0.05)
- Fix partiel ordre opérations Start/Stop
- Créé ce plan d'implémentation

### Fichiers modifiés cette session
- `src-tauri/src/audio/capture.rs` - Fix ordre Start/Stop
- `src-tauri/src/audio/processor.rs` - Ajout normalize_audio()
- `src-tauri/src/audio/mod.rs` - Export normalize_audio
- `src-tauri/src/commands/transcription.rs` - Appel normalisation
- `src-tauri/src/commands/test_transcription.rs` - Appel normalisation

### 2026-02-03 (Phase 0 Fix)
- **Bug identifié**: Race condition TOCTOU - callbacks audio "stale" pouvaient écrire dans le buffer du nouvel enregistrement
- **Solution**: Implémentation compteur de génération (`recording_generation: Arc<AtomicU64>`)
- Chaque enregistrement incrémente le compteur
- Les callbacks vérifient leur génération avant d'écrire
- Logs diagnostics ajoutés (debug pour callbacks stale, info pour start/stop)

**Fichiers modifiés**:
- `src-tauri/src/audio/capture.rs` - Ajout compteur de génération et vérification dans callbacks

### 2026-02-03 (Phase 1 - Chunking)
- **Créé** `src-tauri/src/audio/chunker.rs` - Découpage audio avec overlap
- **Créé** `src-tauri/src/engine/merger.rs` - Fusion transcriptions
- **Modifié** `src-tauri/src/engine/parakeet.rs` - Support chunking automatique
- **Modifié** `src-tauri/src/audio/mod.rs` - Export chunker
- **Modifié** `src-tauri/src/engine/mod.rs` - Export merger

**Configuration chunking**:
- Chunk size: 10 secondes
- Overlap: 2 secondes
- Step: 8 secondes (10 - 2)

**Algorithme de fusion**:
- Premier chunk: garde tout sauf fin overlap
- Dernier chunk: ignore début overlap
- Chunks milieu: trim les deux côtés
