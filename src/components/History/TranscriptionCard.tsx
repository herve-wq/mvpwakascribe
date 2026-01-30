import type { Transcription } from "../../lib/types";

interface TranscriptionCardProps {
  transcription: Transcription;
  onOpen: () => void;
  onDelete: () => void;
}

function formatDuration(ms: number): string {
  const totalSeconds = Math.floor(ms / 1000);
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  return `${minutes}:${seconds.toString().padStart(2, "0")}`;
}

export function TranscriptionCard({
  transcription,
  onOpen,
  onDelete,
}: TranscriptionCardProps) {
  const preview =
    transcription.rawText.slice(0, 100) +
    (transcription.rawText.length > 100 ? "..." : "");

  const date = new Date(transcription.createdAt);
  const time = date.toLocaleTimeString("fr-FR", {
    hour: "2-digit",
    minute: "2-digit",
  });

  return (
    <div className="bg-[var(--color-bg-tertiary)] rounded-lg p-3 hover:bg-[var(--color-border)] transition-colors">
      <div className="flex items-start justify-between gap-2">
        <div className="flex items-center gap-2">
          {transcription.sourceType === "dictation" ? (
            <svg
              className="w-4 h-4 text-[var(--color-text-muted)] shrink-0"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M19 11a7 7 0 01-7 7m0 0a7 7 0 01-7-7m7 7v4m0 0H8m4 0h4m-4-8a3 3 0 01-3-3V5a3 3 0 116 0v6a3 3 0 01-3 3z"
              />
            </svg>
          ) : (
            <svg
              className="w-4 h-4 text-[var(--color-text-muted)] shrink-0"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M9 19V6l12-3v13M9 19c0 1.105-1.343 2-3 2s-3-.895-3-2 1.343-2 3-2 3 .895 3 2zm12-3c0 1.105-1.343 2-3 2s-3-.895-3-2 1.343-2 3-2 3 .895 3 2zM9 10l12-3"
              />
            </svg>
          )}
          <span className="text-sm font-medium text-[var(--color-text-primary)] truncate">
            {transcription.sourceName || "Dictee"}
          </span>
        </div>
        <div className="flex items-center gap-1 text-xs text-[var(--color-text-muted)]">
          <span>{time}</span>
          <span>-</span>
          <span>{formatDuration(transcription.durationMs)}</span>
        </div>
      </div>

      <p className="mt-2 text-xs text-[var(--color-text-secondary)] line-clamp-2">
        {preview || "Transcription vide"}
      </p>

      <div className="mt-3 flex items-center gap-2">
        <button
          onClick={onOpen}
          className="text-xs text-[var(--color-accent)] hover:underline"
        >
          Ouvrir
        </button>
        <span className="text-[var(--color-border)]">|</span>
        <button
          onClick={onDelete}
          className="text-xs text-[var(--color-error)] hover:underline"
        >
          Supprimer
        </button>
      </div>
    </div>
  );
}
