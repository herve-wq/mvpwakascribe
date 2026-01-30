import { useRecording } from "../../hooks/useRecording";

function formatTime(ms: number): string {
  const totalSeconds = Math.floor(ms / 1000);
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  return `${minutes.toString().padStart(2, "0")}:${seconds.toString().padStart(2, "0")}`;
}

export function RecordingControls() {
  const { recordingState, elapsedMs, toggleRecording, togglePause } =
    useRecording();

  const isRecording = recordingState === "recording";
  const isPaused = recordingState === "paused";
  const isProcessing = recordingState === "processing";
  const isActive = isRecording || isPaused;

  return (
    <div className="flex items-center justify-center gap-4">
      {/* Timer */}
      <div className="w-24 text-center">
        {isActive && (
          <div className="flex items-center gap-2">
            <span
              className={`w-2 h-2 rounded-full ${
                isRecording ? "bg-red-500 animate-pulse" : "bg-yellow-500"
              }`}
            />
            <span className="font-mono text-lg text-[var(--color-text-primary)]">
              {formatTime(elapsedMs)}
            </span>
          </div>
        )}
      </div>

      {/* Main record/stop button */}
      <button
        onClick={toggleRecording}
        disabled={isProcessing}
        className={`w-16 h-16 rounded-full flex items-center justify-center transition-all ${
          isProcessing
            ? "bg-gray-400 cursor-not-allowed"
            : isActive
              ? "bg-red-500 hover:bg-red-600"
              : "bg-[var(--color-accent)] hover:bg-[var(--color-accent-hover)]"
        }`}
      >
        {isProcessing ? (
          <svg
            className="w-6 h-6 text-white animate-spin"
            fill="none"
            viewBox="0 0 24 24"
          >
            <circle
              className="opacity-25"
              cx="12"
              cy="12"
              r="10"
              stroke="currentColor"
              strokeWidth="4"
            />
            <path
              className="opacity-75"
              fill="currentColor"
              d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
            />
          </svg>
        ) : isActive ? (
          <svg className="w-6 h-6 text-white" fill="currentColor" viewBox="0 0 24 24">
            <rect x="6" y="6" width="12" height="12" rx="2" />
          </svg>
        ) : (
          <svg className="w-6 h-6 text-white" fill="currentColor" viewBox="0 0 24 24">
            <circle cx="12" cy="12" r="6" />
          </svg>
        )}
      </button>

      {/* Pause button */}
      <div className="w-24">
        {isActive && (
          <button
            onClick={togglePause}
            className="p-3 rounded-full bg-[var(--color-bg-tertiary)] hover:bg-[var(--color-border)] transition-colors"
          >
            {isPaused ? (
              <svg
                className="w-5 h-5 text-[var(--color-text-primary)]"
                fill="currentColor"
                viewBox="0 0 24 24"
              >
                <path d="M8 5v14l11-7z" />
              </svg>
            ) : (
              <svg
                className="w-5 h-5 text-[var(--color-text-primary)]"
                fill="currentColor"
                viewBox="0 0 24 24"
              >
                <rect x="6" y="4" width="4" height="16" rx="1" />
                <rect x="14" y="4" width="4" height="16" rx="1" />
              </svg>
            )}
          </button>
        )}
      </div>
    </div>
  );
}
