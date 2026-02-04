// Core domain types

// Language selection for transcription
export type TranscriptionLanguage = "auto" | "french" | "english";

export const TRANSCRIPTION_LANGUAGES: { value: TranscriptionLanguage; label: string }[] = [
  { value: "auto", label: "Auto" },
  { value: "french", label: "Français" },
  { value: "english", label: "English" },
];

// Decoding configuration for beam search and temperature
export interface DecodingConfig {
  beam_width: number;
  temperature: number;
  blank_penalty: number;
}

// Transcription settings (stored in app settings)
export interface TranscriptionSettings {
  language: TranscriptionLanguage;
  beamWidth: number;      // 1 = greedy (fast), 5 = beam search (quality)
  temperature: number;    // 0.1-1.5, default 1.0
  blankPenalty: number;   // 0-15, default 6.0
}

export const DEFAULT_TRANSCRIPTION_SETTINGS: TranscriptionSettings = {
  language: "auto",
  beamWidth: 1,
  temperature: 1.0,
  blankPenalty: 6.0,
};

export interface Segment {
  id: string;
  startMs: number;
  endMs: number;
  text: string;
  confidence: number;
}

export interface Transcription {
  id: string;
  createdAt: string;
  updatedAt: string;
  sourceType: "dictation" | "file";
  sourceName?: string;
  durationMs: number;
  language: string;
  segments: Segment[];
  rawText: string;
  editedText?: string;
  isEdited: boolean;
}

export interface AudioDevice {
  id: string;
  name: string;
  isDefault: boolean;
}

// Available inference engine backends
export type EngineBackend = "openvino" | "onnxruntime" | "coreml";

export const ENGINE_BACKENDS: { value: EngineBackend; label: string; description: string }[] = [
  { value: "openvino", label: "OpenVINO", description: "Intel OpenVINO (default, optimized for Intel CPUs)" },
  { value: "onnxruntime", label: "ONNX Runtime", description: "Microsoft ONNX Runtime (cross-platform)" },
  { value: "coreml", label: "CoreML", description: "Apple CoreML (optimized for Apple Silicon, Neural Engine)" },
];

export interface Settings {
  theme: "light" | "dark" | "system";
  language: string;
  inputDeviceId?: string;
  shortcuts: {
    toggleRecording: string;
    pause: string;
    copy: string;
  };
  transcription: TranscriptionSettings;
  engineBackend: EngineBackend;
}

export type RecordingState = "idle" | "recording" | "paused" | "processing";

export type TranscriptionMode = "dictation" | "file";

export interface TranscriptionProgress {
  currentMs: number;
  totalMs: number;
  speedFactor: number;
}

// Tauri command return types
export interface TranscribeResult {
  transcription: Transcription;
}

export interface StreamingSegment {
  text: string;
  isFinal: boolean;
  confidence?: number;
}
