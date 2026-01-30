import type { TranscriptionProgress } from "../../lib/types";

interface ProgressBarProps {
  fileName: string;
  progress: TranscriptionProgress;
  onCancel?: () => void;
}

function formatTime(ms: number): string {
  const totalSeconds = Math.floor(ms / 1000);
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  return `${minutes.toString().padStart(2, "0")}:${seconds.toString().padStart(2, "0")}`;
}

export function ProgressBar({ fileName, progress, onCancel }: ProgressBarProps) {
  const percentage = Math.round((progress.currentMs / progress.totalMs) * 100);

  return (
    <div className="bg-[var(--color-bg-secondary)] rounded-lg border border-[var(--color-border)] p-4">
      <div className="flex items-center justify-between mb-2">
        <div className="flex items-center gap-2">
          <svg
            className="w-5 h-5 text-[var(--color-text-muted)]"
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
          <span className="text-sm font-medium text-[var(--color-text-primary)] truncate max-w-xs">
            {fileName}
          </span>
        </div>
        {onCancel && (
          <button
            onClick={onCancel}
            className="p-1 rounded hover:bg-[var(--color-bg-tertiary)]"
          >
            <svg
              className="w-4 h-4 text-[var(--color-text-muted)]"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M6 18L18 6M6 6l12 12"
              />
            </svg>
          </button>
        )}
      </div>

      {/* Progress bar */}
      <div className="h-2 bg-[var(--color-bg-tertiary)] rounded-full overflow-hidden">
        <div
          className="h-full bg-[var(--color-accent)] transition-all duration-300"
          style={{ width: `${percentage}%` }}
        />
      </div>

      {/* Stats */}
      <div className="flex items-center justify-between mt-2 text-xs text-[var(--color-text-muted)]">
        <span>{percentage}%</span>
        <span>
          {formatTime(progress.currentMs)} / {formatTime(progress.totalMs)}
        </span>
        <span>{progress.speedFactor.toFixed(1)}x temps reel</span>
      </div>
    </div>
  );
}
