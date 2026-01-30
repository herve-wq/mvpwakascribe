import { useState } from "react";
import { DropZone } from "./DropZone";
import { ProgressBar } from "./ProgressBar";
import { useTranscription } from "../../hooks/useTranscription";
import type { Transcription, TranscriptionProgress } from "../../lib/types";

export function FileTranscribe() {
  const [isProcessing, setIsProcessing] = useState(false);
  const [currentFile, setCurrentFile] = useState<string | null>(null);
  const [progress, setProgress] = useState<TranscriptionProgress | null>(null);
  const [result, setResult] = useState<Transcription | null>(null);
  const { transcribeFile, copyText } = useTranscription();

  const handleFileSelect = async (path: string) => {
    setCurrentFile(path);
    setIsProcessing(true);
    setProgress({ currentMs: 0, totalMs: 1, speedFactor: 0 });
    setResult(null);

    const transcription = await transcribeFile(path, (p) => {
      setProgress(p);
    });

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
      {/* Drop zone or progress */}
      {!isProcessing && !result ? (
        <DropZone onFileSelect={handleFileSelect} />
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
