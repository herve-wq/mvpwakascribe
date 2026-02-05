import { useCallback } from "react";
import { listen } from "@tauri-apps/api/event";
import { useAppStore } from "../stores/appStore";
import {
  listTranscriptions,
  getTranscription,
  deleteTranscription as tauriDeleteTranscription,
  deleteAllTranscriptions as tauriDeleteAllTranscriptions,
  updateTranscriptionText,
  transcribeFile as tauriTranscribeFile,
  exportToTxt,
  exportToDocx,
  copyToClipboard,
} from "../lib/tauri";
import type { Transcription, TranscriptionProgress, DecodingConfig } from "../lib/types";

export function useTranscription() {
  const { transcriptions, setTranscriptions, addTranscription, settings } = useAppStore();

  // Build DecodingConfig from settings
  const getDecodingConfig = useCallback((): DecodingConfig => ({
    beam_width: settings.transcription.beamWidth,
    temperature: settings.transcription.temperature,
    blank_penalty: settings.transcription.blankPenalty,
  }), [settings.transcription]);

  const loadTranscriptions = useCallback(async () => {
    try {
      const list = await listTranscriptions();
      setTranscriptions(list);
    } catch (error) {
      console.error("Failed to load transcriptions:", error);
    }
  }, [setTranscriptions]);

  const transcribeFile = useCallback(
    async (
      filePath: string,
      onProgress?: (progress: TranscriptionProgress) => void
    ): Promise<Transcription | null> => {
      try {
        // Set up progress listener
        let unlisten: (() => void) | null = null;
        if (onProgress) {
          unlisten = await listen<TranscriptionProgress>(
            "transcription-progress",
            (event) => {
              onProgress(event.payload);
            }
          );
        }

        // Use global settings for language and decoding config
        const language = settings.transcription.language;
        const decodingConfig = getDecodingConfig();

        const transcription = await tauriTranscribeFile(filePath, language, decodingConfig);
        addTranscription(transcription);

        if (unlisten) {
          unlisten();
        }

        return transcription;
      } catch (error) {
        console.error("Failed to transcribe file:", error);
        return null;
      }
    },
    [addTranscription, settings.transcription, getDecodingConfig]
  );

  const deleteTranscription = useCallback(
    async (id: string) => {
      try {
        await tauriDeleteTranscription(id);
        setTranscriptions(transcriptions.filter((t) => t.id !== id));
      } catch (error) {
        console.error("Failed to delete transcription:", error);
      }
    },
    [transcriptions, setTranscriptions]
  );

  const deleteAllTranscriptions = useCallback(async () => {
    try {
      await tauriDeleteAllTranscriptions();
      setTranscriptions([]);
    } catch (error) {
      console.error("Failed to delete all transcriptions:", error);
    }
  }, [setTranscriptions]);

  const updateText = useCallback(async (id: string, editedText: string) => {
    try {
      await updateTranscriptionText(id, editedText);
    } catch (error) {
      console.error("Failed to update transcription:", error);
    }
  }, []);

  const exportTxt = useCallback(async (id: string, path: string) => {
    try {
      await exportToTxt(id, path);
    } catch (error) {
      console.error("Failed to export to txt:", error);
    }
  }, []);

  const exportDocx = useCallback(async (id: string, path: string) => {
    try {
      await exportToDocx(id, path);
    } catch (error) {
      console.error("Failed to export to docx:", error);
    }
  }, []);

  const copyText = useCallback(async (text: string) => {
    try {
      await copyToClipboard(text);
    } catch (error) {
      console.error("Failed to copy to clipboard:", error);
    }
  }, []);

  return {
    transcriptions,
    loadTranscriptions,
    getTranscription,
    transcribeFile,
    deleteTranscription,
    deleteAllTranscriptions,
    updateText,
    exportTxt,
    exportDocx,
    copyText,
    // Expose transcription settings for components that need them
    transcriptionSettings: settings.transcription,
    getDecodingConfig,
  };
}
