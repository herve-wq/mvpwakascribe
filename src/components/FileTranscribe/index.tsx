import { useState, useEffect } from "react";
import { DropZone } from "./DropZone";
import { ProgressBar } from "./ProgressBar";
import { useTranscription } from "../../hooks/useTranscription";
import { checkTestAudio, startRecording, stopRecordingToWav } from "../../lib/tauri";
import type { Transcription, TranscriptionProgress, TranscriptionLanguage } from "../../lib/types";
import { TRANSCRIPTION_LANGUAGES } from "../../lib/types";

export function FileTranscribe() {
  const [isProcessing, setIsProcessing] = useState(false);
  const [currentFile, setCurrentFile] = useState<string | null>(null);
  const [progress, setProgress] = useState<TranscriptionProgress | null>(null);
  const [result, setResult] = useState<Transcription | null>(null);
  const { transcribeFile, copyText } = useTranscription();

  // Recording state for test audio
  const [isRecording, setIsRecording] = useState(false);
  const [recordingError, setRecordingError] = useState<string | null>(null);

  // Language selection
  const [selectedLanguage, setSelectedLanguage] = useState<TranscriptionLanguage>("auto");

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

    const transcription = await transcribeFile(
      path,
      (p) => setProgress(p),
      selectedLanguage
    );

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

  return (
    <div className="h-full flex flex-col p-6 gap-6">
      {/* Language selector */}
      {!isProcessing && !result && (
        <div className="flex items-center gap-3">
          <label className="text-sm text-[var(--color-text-secondary)]">
            Langue:
          </label>
          <select
            value={selectedLanguage}
            onChange={(e) => setSelectedLanguage(e.target.value as TranscriptionLanguage)}
            className="px-3 py-1.5 rounded-lg bg-[var(--color-bg-secondary)] border border-[var(--color-border)]
                       text-[var(--color-text-primary)] text-sm
                       focus:outline-none focus:ring-2 focus:ring-[var(--color-accent)] focus:border-transparent"
          >
            {TRANSCRIPTION_LANGUAGES.map((lang) => (
              <option key={lang.value} value={lang.value}>
                {lang.label}
              </option>
            ))}
          </select>
        </div>
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
