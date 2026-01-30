# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

WakaScribe is an offline speech-to-text desktop application for macOS Intel, using NVIDIA Parakeet TDT model. Target: MacBook Pro Intel (2019).

## Tech Stack

- **Framework**: Tauri 2.x (Rust backend + web frontend)
- **Frontend**: React 19 + TypeScript + Tailwind CSS v4
- **Backend**: Rust
- **STT Engine**: Parakeet TDT via openvino
- **Database**: SQLite via rusqlite
- **Audio**: cpal (Rust)

## Prerequisites

### OpenVINO
Install OpenVINO via Homebrew (required for ONNX model inference):
```bash
brew install openvino
```

## Build Commands

```bash
# Development (runs both frontend and backend)
npm run tauri:dev

# Build production DMG
npm run tauri:build

# Frontend only
npm run dev
npm run build

# Check Rust code
cargo check
```

Note: The `tauri:dev` and `tauri:build` scripts automatically set `OPENVINO_LIB_PATH=/usr/local/lib` for proper library loading.

## Project Structure

```
wakascribe/
├── src/                          # React frontend
│   ├── components/
│   │   ├── Layout.tsx           # Main layout with sidebar panels
│   │   ├── TitleBar.tsx         # Custom window title bar
│   │   ├── Recorder/            # Dictation mode components
│   │   ├── FileTranscribe/      # File transcription components
│   │   ├── Editor/              # Transcription editor
│   │   ├── History/             # History panel
│   │   └── Settings/            # Settings panel
│   ├── hooks/                   # React hooks (useRecording, useTheme, etc.)
│   ├── stores/appStore.ts       # Zustand state management
│   └── lib/
│       ├── tauri.ts             # Tauri command wrappers
│       └── types.ts             # TypeScript types
├── src-tauri/                   # Rust backend
│   ├── src/
│   │   ├── lib.rs               # Main entry, plugin setup
│   │   ├── commands/            # Tauri command handlers
│   │   ├── audio/               # Audio capture via cpal (threaded)
│   │   ├── engine/              # Parakeet STT inference
│   │   │   ├── parakeet.rs      # ONNX model loading and inference via tract
│   │   │   ├── mel.rs           # Mel spectrogram computation
│   │   │   └── decoder.rs       # TDT token decoding
│   │   ├── storage/             # SQLite database
│   │   └── export/              # TXT and DOCX export
│   └── migrations/001_init.sql  # Database schema
├── model/                        # ONNX model files
│   ├── encoder-model.onnx       # Conformer encoder
│   ├── decoder_joint-model.onnx # TDT decoder
│   ├── config.json              # Model configuration
│   └── vocab.txt                # Vocabulary (8193 tokens)
└── PRD_WakaScribe_MVP.md        # Product requirements
```

## Key Implementation Details

### ONNX Inference Engine (src-tauri/src/engine/)

Uses OpenVINO for ONNX model inference (requires Homebrew openvino package):
- `parakeet.rs`: Loads encoder and decoder ONNX models via OpenVINO, runs inference pipeline
- `mel.rs`: Computes 128-mel spectrogram from 16kHz audio (512 FFT, 160 hop)
- `decoder.rs`: TDT decoder converts token IDs to text with timestamps

Model configuration (from config.json):
- `features_size`: 128 mel features
- `subsampling_factor`: 8 (Conformer architecture)

### Audio Capture (src-tauri/src/audio/capture.rs)
- Uses a dedicated thread to handle cpal::Stream (not Send-safe)
- Commands sent via mpsc channel to audio thread
- AudioCapture struct is Send+Sync safe for Tauri state

### Tauri Commands
Commands are defined in `src-tauri/src/commands/` and exposed in `lib.rs`:
- `list_audio_devices`, `start_recording`, `stop_recording`, `pause_recording`, `resume_recording`
- `transcribe_file`, `get_transcription`
- `list_transcriptions`, `delete_transcription`, `update_transcription_text`
- `get_settings`, `update_settings`
- `export_to_txt`, `export_to_docx`, `copy_to_clipboard`

### Database
SQLite database stored at `~/Library/Application Support/com.wakascribe.desktop/wakascribe.db`

## Model Files

The ONNX model files should be placed in the `model/` directory:
- `encoder-model.onnx` - NeMo Conformer encoder
- `encoder-model.onnx.data` - External weights for encoder (large file)
- `decoder_joint-model.onnx` - TDT joint decoder
- `vocab.txt` - Vocabulary file
- `config.json` - Model configuration

## Important Notes

### Tailwind CSS v4
- Uses CSS-based config in `src/index.css` (no tailwind.config.js)
- **Critical**: Must exclude binary directories with `@source not` directives to prevent scanning model/ONNX files:
  ```css
  @source not "../model";
  @source not "../src-tauri";
  ```

### Model Loading
- Model is loaded at app startup in `lib.rs`
- If model files are missing, app falls back to mock transcription
- Model path is resolved relative to executable or from bundle Resources

### App Identifier
`com.wakascribe.desktop`
