import { create } from "zustand";
import type {
  RecordingState,
  TranscriptionMode,
  Transcription,
  Segment,
  Settings,
  AudioDevice,
} from "../lib/types";

interface AppState {
  // Recording state
  recordingState: RecordingState;
  currentMode: TranscriptionMode;
  elapsedMs: number;

  // Current transcription
  currentSegments: Segment[];
  pendingText: string;

  // Audio
  audioDevices: AudioDevice[];
  selectedDeviceId: string | null;
  audioLevel: number;

  // Settings
  settings: Settings;

  // History
  transcriptions: Transcription[];

  // UI
  showSettings: boolean;
  showHistory: boolean;

  // Actions
  setRecordingState: (state: RecordingState) => void;
  setCurrentMode: (mode: TranscriptionMode) => void;
  setElapsedMs: (ms: number) => void;
  addSegment: (segment: Segment) => void;
  setPendingText: (text: string) => void;
  clearCurrentTranscription: () => void;
  setAudioDevices: (devices: AudioDevice[]) => void;
  setSelectedDeviceId: (id: string | null) => void;
  setAudioLevel: (level: number) => void;
  setSettings: (settings: Partial<Settings>) => void;
  setTranscriptions: (transcriptions: Transcription[]) => void;
  addTranscription: (transcription: Transcription) => void;
  toggleSettings: () => void;
  toggleHistory: () => void;
}

const defaultSettings: Settings = {
  theme: "system",
  language: "fr",
  shortcuts: {
    toggleRecording: "CommandOrControl+Shift+R",
    pause: "CommandOrControl+Shift+P",
    copy: "CommandOrControl+Shift+C",
  },
};

export const useAppStore = create<AppState>((set) => ({
  // Initial state
  recordingState: "idle",
  currentMode: "dictation",
  elapsedMs: 0,
  currentSegments: [],
  pendingText: "",
  audioDevices: [],
  selectedDeviceId: null,
  audioLevel: 0,
  settings: defaultSettings,
  transcriptions: [],
  showSettings: false,
  showHistory: false,

  // Actions
  setRecordingState: (recordingState) => set({ recordingState }),
  setCurrentMode: (currentMode) => set({ currentMode }),
  setElapsedMs: (elapsedMs) => set({ elapsedMs }),

  addSegment: (segment) =>
    set((state) => ({
      currentSegments: [...state.currentSegments, segment],
    })),

  setPendingText: (pendingText) => set({ pendingText }),

  clearCurrentTranscription: () =>
    set({
      currentSegments: [],
      pendingText: "",
      elapsedMs: 0,
    }),

  setAudioDevices: (audioDevices) => set({ audioDevices }),
  setSelectedDeviceId: (selectedDeviceId) => set({ selectedDeviceId }),
  setAudioLevel: (audioLevel) => set({ audioLevel }),

  setSettings: (newSettings) =>
    set((state) => ({
      settings: { ...state.settings, ...newSettings },
    })),

  setTranscriptions: (transcriptions) => set({ transcriptions }),

  addTranscription: (transcription) =>
    set((state) => ({
      transcriptions: [transcription, ...state.transcriptions],
    })),

  toggleSettings: () => set((state) => ({ showSettings: !state.showSettings })),
  toggleHistory: () => set((state) => ({ showHistory: !state.showHistory })),
}));
