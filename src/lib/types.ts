// Core domain types

// Language selection for transcription
export type TranscriptionLanguage = "auto" | "french" | "english";

export const TRANSCRIPTION_LANGUAGES: { value: TranscriptionLanguage; label: string }[] = [
  { value: "auto", label: "Auto" },
  { value: "french", label: "Français" },
  { value: "english", label: "English" },
];

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

export interface Settings {
  theme: "light" | "dark" | "system";
  language: string;
  inputDeviceId?: string;
  shortcuts: {
    toggleRecording: string;
    pause: string;
    copy: string;
  };
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
