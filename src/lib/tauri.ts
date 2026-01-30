import { invoke } from "@tauri-apps/api/core";
import type {
  AudioDevice,
  Transcription,
  Settings,
  TranscriptionProgress,
} from "./types";

// Audio commands
export async function listAudioDevices(): Promise<AudioDevice[]> {
  return invoke("list_audio_devices");
}

export async function startRecording(deviceId?: string): Promise<void> {
  return invoke("start_recording", { deviceId });
}

export async function stopRecording(): Promise<Transcription> {
  return invoke("stop_recording");
}

export async function pauseRecording(): Promise<void> {
  return invoke("pause_recording");
}

export async function resumeRecording(): Promise<void> {
  return invoke("resume_recording");
}

export async function getAudioLevel(): Promise<number> {
  return invoke("get_audio_level");
}

// File transcription commands
export async function transcribeFile(
  filePath: string,
  _onProgress?: (progress: TranscriptionProgress) => void
): Promise<Transcription> {
  // Progress updates come through Tauri events (handled via listen())
  return invoke("transcribe_file", { filePath });
}

// History commands
export async function listTranscriptions(): Promise<Transcription[]> {
  return invoke("list_transcriptions");
}

export async function getTranscription(id: string): Promise<Transcription> {
  return invoke("get_transcription", { id });
}

export async function deleteTranscription(id: string): Promise<void> {
  return invoke("delete_transcription", { id });
}

export async function updateTranscriptionText(
  id: string,
  editedText: string
): Promise<void> {
  return invoke("update_transcription_text", { id, editedText });
}

// Settings commands
export async function getSettings(): Promise<Settings> {
  return invoke("get_settings");
}

export async function updateSettings(settings: Partial<Settings>): Promise<void> {
  return invoke("update_settings", { settings });
}

// Export commands
export async function exportToTxt(id: string, path: string): Promise<void> {
  return invoke("export_to_txt", { id, path });
}

export async function exportToDocx(id: string, path: string): Promise<void> {
  return invoke("export_to_docx", { id, path });
}

export async function copyToClipboard(text: string): Promise<void> {
  return invoke("copy_to_clipboard", { text });
}
