import { useState, useEffect } from "react";
import { DropZone } from "./DropZone";
import { ProgressBar } from "./ProgressBar";
import { useTranscription } from "../../hooks/useTranscription";
import { useAppStore } from "../../stores/appStore";
import { checkTestAudio, startRecording, stopRecordingToWav } from "../../lib/tauri";
import type { Transcription, TranscriptionProgress } from "../../lib/types";
import { TRANSCRIPTION_LANGUAGES } from "../../lib/types";

export function FileTranscribe() {
  const [isProcessing, setIsProcessing] = useState(false);
  const [currentFile, setCurrentFile] = useState<string | null>(null);
  const [progress, setProgress] = useState<TranscriptionProgress | null>(null);
  const [result, setResult] = useState<Transcription | null>(null);
  const { transcribeFile, copyText, transcriptionSettings } = useTranscription();
  const { toggleSettings } = useAppStore();

  // Recording state for test audio
  const [isRecording, setIsRecording] = useState(false);
  const [recordingError, setRecordingError] = useState<string | null>(null);

  // Test file path - pour désactiver, commenter ce bloc
  const [testFilePath, setTestFilePath] = useState<string | null>(null);
  useEffect(() => {
    checkTestAudio()
      .then(setTestFilePath)
      .catch(() => setTestFilePath(null));
  }, []);

  // Toggle recording for test audio
  const handleToggleRecording = async () => {
    setRecordingError(null);
    try {
      if (isRecording) {
        // Stop recording and save WAV
        const savedPath = await stopRecordingToWav();
        setIsRecording(false);
        setTestFilePath(savedPath);
      } else {
        // Start recording
        await startRecording();
        setIsRecording(true);
      }
    } catch (error) {
      setRecordingError(error instanceof Error ? error.message : String(error));
      setIsRecording(false);
    }
  };

  const handleTestFile = () => {
    if (testFilePath) {
      handleFileSelect(testFilePath);
    }
  };
  // Fin du bloc test

  const handleFileSelect = async (path: string) => {
    setCurrentFile(path);
    setIsProcessing(true);
    setProgress({ currentMs: 0, totalMs: 1, speedFactor: 0 });
    setResult(null);

    // transcribeFile now uses global settings from useTranscription hook
    const transcription = await transcribeFile(path, (p) => setProgress(p));

    setIsProcessing(false);
    setProgress(null);

    if (transcription) {
      setResult(transcription);
    }
  };

  const handleCopy = () => {
    if (result) {
      copyText(result.rawText);
    }
  };

  const handleReset = () => {
    setCurrentFile(null);
    setResult(null);
  };

  const fileName = currentFile?.split("/").pop() || "";

  // Get language label for display
  const languageLabel = TRANSCRIPTION_LANGUAGES.find(
    (l) => l.value === transcriptionSettings.language
  )?.label || "Auto";

  return (
    <div className="h-full flex flex-col p-6 gap-6">
      {/* Settings summary - clickable to open settings */}
      {!isProcessing && !result && (
        <button
          onClick={toggleSettings}
          className="flex items-center justify-between px-4 py-3 rounded-lg bg-[var(--color-bg-secondary)]
                     border border-[var(--color-border)] hover:border-[var(--color-accent)] transition-colors
                     text-left group"
        >
          <div className="flex items-center gap-3">
            <svg
              className="w-5 h-5 text-[var(--color-text-muted)] group-hover:text-[var(--color-accent)]"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"
              />
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"
              />
            </svg>
            <div>
              <span className="text-sm font-medium text-[var(--color-text-primary)]">
                {languageLabel}
              </span>
              <span className="text-sm text-[var(--color-text-muted)]"> · </span>
              <span className="text-sm text-[var(--color-text-muted)]">
                {transcriptionSettings.beamWidth > 1 ? "Precis" : "Rapide"}
              </span>
              <span className="text-sm text-[var(--color-text-muted)]"> · </span>
              <span className="text-sm text-[var(--color-text-muted)] font-mono">
                temp={transcriptionSettings.temperature.toFixed(1)}
              </span>
            </div>
          </div>
          <svg
            className="w-4 h-4 text-[var(--color-text-muted)] group-hover:text-[var(--color-accent)]"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M9 5l7 7-7 7"
            />
          </svg>
        </button>
      )}

      {/* Drop zone or progress */}
      {!isProcessing && !result ? (
        <div className="space-y-4">
          <DropZone onFileSelect={handleFileSelect} />

          {/* Bouton enregistrement test */}
          <button
            onClick={handleToggleRecording}
            disabled={isProcessing}
            className={`w-full px-4 py-3 rounded-lg border-2 border-dashed transition-all flex items-center justify-center gap-3
              ${isRecording
                ? "border-red-500 bg-red-500/10 hover:bg-red-500/20"
                : "border-green-500/50 bg-green-500/5 hover:bg-green-500/10 hover:border-green-500"
              }
              ${isProcessing ? "opacity-50 cursor-not-allowed" : ""}
            `}
          >
            {isRecording ? (
              <>
                <span className="relative flex h-4 w-4">
                  <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-red-400 opacity-75"></span>
                  <span className="relative inline-flex rounded-full h-4 w-4 bg-red-500"></span>
                </span>
                <span className="text-red-600 dark:text-red-400 font-medium">
                  Arrêter l'enregistrement
                </span>
              </>
            ) : (
              <>
                <svg
                  className="w-5 h-5 text-green-500"
                  fill="currentColor"
                  viewBox="0 0 24 24"
                >
                  <circle cx="12" cy="12" r="8" />
                </svg>
                <span className="text-green-600 dark:text-green-400 font-medium">
                  Enregistrer test_audio.wav
                </span>
              </>
            )}
          </button>
          {recordingError && (
            <p className="text-sm text-red-500 text-center">{recordingError}</p>
          )}
          {/* Fin bouton enregistrement test */}

          {/* Bouton fichier test - pour désactiver, supprimer ce bloc */}
          {testFilePath && (
            <button
              onClick={handleTestFile}
              disabled={isRecording}
              className={`w-full px-4 py-3 rounded-lg border-2 border-dashed border-blue-500/50
                         bg-blue-500/5 hover:bg-blue-500/10 hover:border-blue-500
                         transition-all flex items-center justify-center gap-3
                         ${isRecording ? "opacity-50 cursor-not-allowed" : ""}`}
            >
              <svg
                className="w-5 h-5 text-blue-500"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2m-6 9l2 2 4-4"
                />
              </svg>
              <span className="text-blue-600 dark:text-blue-400 font-medium">
                Fichier Test (test_audio.wav)
              </span>
            </button>
          )}
          {/* Fin bouton fichier test */}
        </div>
      ) : isProcessing && progress ? (
        <ProgressBar fileName={fileName} progress={progress} />
      ) : null}

      {/* Result display */}
      {result && (
        <>
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <svg
                className="w-5 h-5 text-green-500"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"
                />
              </svg>
              <span className="font-medium text-[var(--color-text-primary)]">
                Transcription terminee
              </span>
            </div>
            <button
              onClick={handleReset}
              className="text-sm text-[var(--color-accent)] hover:underline"
            >
              Nouveau fichier
            </button>
          </div>

          <div className="flex-1 bg-[var(--color-bg-secondary)] rounded-lg border border-[var(--color-border)] overflow-auto">
            <div className="p-4">
              <p className="text-[var(--color-text-primary)] whitespace-pre-wrap">
                {result.rawText}
              </p>
            </div>
          </div>

          <div className="flex items-center justify-end gap-3">
            <button
              onClick={handleCopy}
              className="flex items-center gap-2 px-4 py-2 rounded-lg bg-[var(--color-bg-tertiary)] hover:bg-[var(--color-border)] transition-colors"
            >
              <svg
                className="w-4 h-4"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z"
                />
              </svg>
              <span className="text-sm">Copier</span>
            </button>
          </div>
        </>
      )}
    </div>
  );
}
