# WakaScribe - Project Documentation

Offline speech-to-text desktop application for macOS using NVIDIA Parakeet TDT model.

## Tech Stack

- **Framework**: Tauri 2.x (Rust backend + React frontend)
- **Frontend**: React 19 + TypeScript 5.8 + Tailwind CSS v4 + Zustand
- **Backend**: Rust 2021 edition
- **STT Engines**: OpenVINO (primary), ONNX Runtime, CoreML
- **Database**: SQLite via rusqlite
- **Audio**: cpal + rubato (resampling)

## Prerequisites

```bash
# OpenVINO (required for primary inference backend)
brew install openvino
```

## Build Commands

```bash
npm run tauri:dev      # Development (frontend + backend)
npm run tauri:build    # Production DMG
npm run dev            # Frontend only
cargo check            # Check Rust code
```

Note: Scripts automatically set `OPENVINO_LIB_PATH=/usr/local/lib`.

## Project Structure

```
wakascribe/
├── src/                          # React frontend
│   ├── App.tsx                   # Main app, mode switching
│   ├── index.css                 # Tailwind v4 theme
│   ├── components/
│   │   ├── Layout.tsx            # Main layout with sidebar
│   │   ├── TitleBar.tsx          # Custom window titlebar
│   │   ├── Recorder/             # Dictation mode
│   │   │   ├── index.tsx
│   │   │   ├── WaveformDisplay.tsx
│   │   │   ├── RecordingControls.tsx
│   │   │   └── ConfidenceIndicator.tsx
│   │   ├── FileTranscribe/       # File transcription
│   │   │   ├── index.tsx
│   │   │   ├── DropZone.tsx
│   │   │   └── ProgressBar.tsx
│   │   ├── Editor/               # Transcription editor
│   │   │   ├── index.tsx
│   │   │   ├── SegmentList.tsx
│   │   │   └── ExportMenu.tsx
│   │   ├── History/              # History panel
│   │   │   ├── index.tsx
│   │   │   ├── SearchBar.tsx
│   │   │   └── TranscriptionCard.tsx
│   │   └── Settings/             # Settings panels
│   │       ├── index.tsx
│   │       ├── AppearanceSettings.tsx
│   │       ├── AudioSettings.tsx
│   │       ├── EngineSettings.tsx
│   │       ├── TranscriptionSettings.tsx
│   │       └── ShortcutSettings.tsx
│   ├── hooks/
│   │   ├── useRecording.ts       # Recording state/control
│   │   ├── useTranscription.ts   # Transcription operations
│   │   ├── useAudioDevices.ts    # Device enumeration
│   │   ├── useSettings.ts        # Settings management
│   │   └── useTheme.ts           # Theme switching
│   ├── stores/
│   │   └── appStore.ts           # Zustand global state
│   └── lib/
│       ├── types.ts              # TypeScript types
│       └── tauri.ts              # Tauri command wrappers
│
├── src-tauri/                    # Rust backend
│   ├── src/
│   │   ├── lib.rs                # Entry, plugin setup, engine init
│   │   ├── main.rs               # Binary entry
│   │   ├── error.rs              # AppError enum
│   │   ├── commands/
│   │   │   ├── mod.rs
│   │   │   ├── audio.rs          # Audio device/recording
│   │   │   ├── transcription.rs  # Transcription commands
│   │   │   ├── history.rs        # History CRUD
│   │   │   ├── settings.rs       # Settings persistence
│   │   │   ├── export.rs         # TXT/DOCX export
│   │   │   └── test_transcription.rs
│   │   ├── audio/
│   │   │   ├── mod.rs
│   │   │   ├── capture.rs        # Live capture (cpal, threaded)
│   │   │   ├── processor.rs      # Resampling, normalization
│   │   │   ├── chunker.rs        # Audio chunking
│   │   │   └── vad.rs            # Voice Activity Detection
│   │   ├── engine/
│   │   │   ├── mod.rs            # DynamicEngine trait
│   │   │   ├── parakeet.rs       # OpenVINO backend
│   │   │   ├── onnxruntime.rs    # ONNX Runtime backend
│   │   │   ├── coreml.rs         # CoreML backend (macOS)
│   │   │   ├── config.rs         # DecodingConfig
│   │   │   ├── mel.rs            # Mel spectrogram
│   │   │   ├── decoder.rs        # TDT beam search decoder
│   │   │   └── merger.rs         # Segment merging
│   │   ├── storage/
│   │   │   ├── mod.rs
│   │   │   ├── database.rs       # DB init/connection
│   │   │   ├── models.rs         # Data models
│   │   │   └── queries.rs        # CRUD operations
│   │   └── export/
│   │       ├── mod.rs
│   │       ├── txt.rs
│   │       └── docx.rs
│   ├── migrations/
│   │   └── 001_init.sql          # DB schema
│   ├── Cargo.toml
│   └── tauri.conf.json
│
├── model/                        # ML models by backend
│   ├── openvino/
│   │   ├── parakeet_encoder.xml/bin
│   │   ├── parakeet_decoder.xml/bin
│   │   ├── parakeet_joint.xml/bin
│   │   ├── parakeet_melspectogram.xml/bin
│   │   └── parakeet_v3_vocab.json
│   ├── onnxruntime/
│   │   ├── encoder-model.int8.onnx
│   │   ├── decoder_joint-model.onnx
│   │   ├── nemo128.onnx
│   │   ├── vocab.txt
│   │   └── config.json
│   └── coreml/
│       ├── Encoder.mlmodelc/
│       ├── Decoder.mlmodelc/
│       ├── Preprocessor.mlmodelc/
│       ├── MelEncoder.mlmodelc/
│       └── parakeet_v3_vocab.json
│
├── package.json
├── vite.config.ts
├── tsconfig.json
└── postcss.config.js
```

## Tauri Commands

**Audio:**
- `list_audio_devices`, `start_recording`, `stop_recording`
- `pause_recording`, `resume_recording`, `get_audio_level`

**Transcription:**
- `transcribe_file`, `get_transcription`

**History:**
- `list_transcriptions`, `delete_transcription`, `update_transcription_text`

**Settings:**
- `get_settings`, `update_settings`
- `switch_engine_backend`, `get_engine_backend`

**Export:**
- `export_to_txt`, `export_to_docx`, `copy_to_clipboard`

## Database Schema

SQLite at `~/Library/Application Support/com.wakascribe.app/wakascribe.db`

```sql
-- Transcriptions table
CREATE TABLE transcriptions (
  id TEXT PRIMARY KEY,
  created_at TEXT, updated_at TEXT,
  source_type TEXT,  -- 'dictation' | 'file'
  source_name TEXT,
  duration_ms INTEGER,
  language TEXT,
  raw_text TEXT, edited_text TEXT, is_edited INTEGER
);

-- Segments table
CREATE TABLE segments (
  id TEXT PRIMARY KEY,
  transcription_id TEXT,
  start_ms INTEGER, end_ms INTEGER,
  text TEXT, confidence REAL
);

-- Settings table
CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT);
```

## Audio Processing Pipeline

1. Capture from device (cpal) → 2. Resample to 16kHz (rubato)
3. Normalize → 4. Mel spectrogram (128 features, 160 hop)
5. Inference (OpenVINO/ONNX/CoreML) → 6. Beam search decode
7. Post-process → 8. Store in SQLite → 9. Stream to frontend

## Model Configuration

- **Features**: 128 mel spectral features
- **Sample rate**: 16kHz
- **Vocabulary**: 8193 tokens + blank
- **Max duration**: 15 seconds (240,000 samples)

## Language Support

- Auto-detection (default)
- Force French: token 71 `<|fr|>`
- Force English: token 64 `<|en|>`

## Decoding Parameters

- `beam_width`: 1 (greedy) to 5+ (quality)
- `temperature`: 0.1-1.5
- `blank_penalty`: 0-15

## Zustand State

```typescript
{
  recordingState: 'idle' | 'recording' | 'paused' | 'processing',
  currentMode: 'dictation' | 'file',
  currentSegments: Segment[],
  settings: Settings,
  transcriptions: Transcription[],
  audioDevices: AudioDevice[],
  audioLevel: number
}
```

## Key Types

```typescript
interface Transcription {
  id: string;
  created_at: string;
  source_type: 'dictation' | 'file';
  source_name: string;
  duration_ms: number;
  language: TranscriptionLanguage;
  segments: Segment[];
  raw_text: string;
  edited_text?: string;
}

interface Segment {
  id: string;
  start_ms: number;
  end_ms: number;
  text: string;
  confidence: number;
}

type EngineBackend = 'openvino' | 'onnxruntime' | 'coreml';
type TranscriptionLanguage = 'auto' | 'french' | 'english';
```

## App Identifier

`com.wakascribe.desktop` (bundle) / `com.wakascribe.app` (storage)

## Tailwind CSS v4

Config in `src/index.css` with `@theme` variables. Must exclude binary dirs:
```css
@source not "../model";
@source not "../src-tauri";
```
