import { useCallback, useEffect, useRef } from "react";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import { useAppStore } from "../stores/appStore";
import {
  startRecording as tauriStartRecording,
  stopRecording as tauriStopRecording,
  pauseRecording as tauriPauseRecording,
  resumeRecording as tauriResumeRecording,
  getAudioLevel as tauriGetAudioLevel,
} from "../lib/tauri";
import type { Segment, StreamingSegment, DecodingConfig } from "../lib/types";

export function useRecording() {
  const {
    recordingState,
    selectedDeviceId,
    elapsedMs,
    currentSegments,
    pendingText,
    settings,
    setRecordingState,
    setElapsedMs,
    addSegment,
    setPendingText,
    setAudioLevel,
    clearCurrentTranscription,
    addTranscription,
  } = useAppStore();

  // Build DecodingConfig from settings
  const getDecodingConfig = useCallback((): DecodingConfig => ({
    beam_width: settings.transcription.beamWidth,
    temperature: settings.transcription.temperature,
    blank_penalty: settings.transcription.blankPenalty,
  }), [settings.transcription]);

  const timerRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const unlistenRefs = useRef<UnlistenFn[]>([]);

  // Set up event listeners for transcription segments
  useEffect(() => {
    async function setupListeners() {
      // Listen for streaming transcription segments
      const unlistenSegment = await listen<StreamingSegment>(
        "transcription-segment",
        (event) => {
          if (event.payload.isFinal) {
            const segment: Segment = {
              id: crypto.randomUUID(),
              startMs: elapsedMs,
              endMs: elapsedMs,
              text: event.payload.text,
              confidence: event.payload.confidence ?? 0.9,
            };
            addSegment(segment);
            setPendingText("");
          } else {
            setPendingText(event.payload.text);
          }
        }
      );

      unlistenRefs.current = [unlistenSegment];
    }

    setupListeners();

    return () => {
      unlistenRefs.current.forEach((unlisten) => unlisten());
    };
  }, [elapsedMs, addSegment, setPendingText]);

  // Poll audio level when recording
  const audioLevelRef = useRef<ReturnType<typeof setInterval> | null>(null);

  useEffect(() => {
    if (recordingState === "recording") {
      audioLevelRef.current = setInterval(async () => {
        try {
          const level = await tauriGetAudioLevel();
          setAudioLevel(level);
        } catch (e) {
          console.error("Failed to get audio level:", e);
        }
      }, 50); // Poll every 50ms for smooth visualization
    } else {
      if (audioLevelRef.current) {
        clearInterval(audioLevelRef.current);
        audioLevelRef.current = null;
      }
      setAudioLevel(0);
    }

    return () => {
      if (audioLevelRef.current) {
        clearInterval(audioLevelRef.current);
      }
    };
  }, [recordingState, setAudioLevel]);

  // Timer for elapsed time
  useEffect(() => {
    if (recordingState === "recording") {
      timerRef.current = setInterval(() => {
        setElapsedMs(elapsedMs + 100);
      }, 100);
    } else if (timerRef.current) {
      clearInterval(timerRef.current);
      timerRef.current = null;
    }

    return () => {
      if (timerRef.current) {
        clearInterval(timerRef.current);
      }
    };
  }, [recordingState, elapsedMs, setElapsedMs]);

  const start = useCallback(async () => {
    try {
      clearCurrentTranscription();
      await tauriStartRecording(selectedDeviceId ?? undefined);
      setRecordingState("recording");
    } catch (error) {
      console.error("Failed to start recording:", error);
      setRecordingState("idle");
    }
  }, [selectedDeviceId, clearCurrentTranscription, setRecordingState]);

  const stop = useCallback(async () => {
    try {
      setRecordingState("processing");
      // Use global settings for language and decoding config
      const language = settings.transcription.language;
      const decodingConfig = getDecodingConfig();
      const transcription = await tauriStopRecording(language, decodingConfig);
      addTranscription(transcription);
      setRecordingState("idle");
      return transcription;
    } catch (error) {
      console.error("Failed to stop recording:", error);
      setRecordingState("idle");
      return null;
    }
  }, [setRecordingState, addTranscription, settings.transcription, getDecodingConfig]);

  const pause = useCallback(async () => {
    try {
      await tauriPauseRecording();
      setRecordingState("paused");
    } catch (error) {
      console.error("Failed to pause recording:", error);
    }
  }, [setRecordingState]);

  const resume = useCallback(async () => {
    try {
      await tauriResumeRecording();
      setRecordingState("recording");
    } catch (error) {
      console.error("Failed to resume recording:", error);
    }
  }, [setRecordingState]);

  const toggleRecording = useCallback(async () => {
    if (recordingState === "idle") {
      await start();
    } else if (recordingState === "recording" || recordingState === "paused") {
      await stop();
    }
  }, [recordingState, start, stop]);

  const togglePause = useCallback(async () => {
    if (recordingState === "recording") {
      await pause();
    } else if (recordingState === "paused") {
      await resume();
    }
  }, [recordingState, pause, resume]);

  return {
    recordingState,
    elapsedMs,
    segments: currentSegments,
    pendingText,
    start,
    stop,
    pause,
    resume,
    toggleRecording,
    togglePause,
  };
}
