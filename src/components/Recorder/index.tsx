import { WaveformDisplay } from "./WaveformDisplay";
import { RecordingControls } from "./RecordingControls";
import { ConfidenceIndicator } from "./ConfidenceIndicator";
import { useRecording } from "../../hooks/useRecording";
import { useAudioDevices } from "../../hooks/useAudioDevices";
import { useTranscription } from "../../hooks/useTranscription";

function formatTimestamp(ms: number): string {
  const totalSeconds = Math.floor(ms / 1000);
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  return `${minutes.toString().padStart(2, "0")}:${seconds.toString().padStart(2, "0")}`;
}

export function Recorder() {
  const { segments, pendingText } = useRecording();
  const { devices, selectedDeviceId, selectDevice } = useAudioDevices();
  const { copyText } = useTranscription();

  const fullText = segments.map((s) => s.text).join(" ");

  const handleCopy = () => {
    if (fullText) {
      copyText(fullText);
    }
  };

  return (
    <div className="h-full flex flex-col p-6 gap-6">
      {/* Waveform */}
      <WaveformDisplay />

      {/* Device selector and status */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
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
              d="M19 11a7 7 0 01-7 7m0 0a7 7 0 01-7-7m7 7v4m0 0H8m4 0h4m-4-8a3 3 0 01-3-3V5a3 3 0 116 0v6a3 3 0 01-3 3z"
            />
          </svg>
          <select
            value={selectedDeviceId || ""}
            onChange={(e) => selectDevice(e.target.value)}
            className="text-sm bg-[var(--color-bg-secondary)] border border-[var(--color-border)] rounded px-2 py-1 text-[var(--color-text-primary)]"
          >
            {devices.map((device) => (
              <option key={device.id} value={device.id}>
                {device.name}
              </option>
            ))}
          </select>
        </div>
      </div>

      {/* Recording controls */}
      <RecordingControls />

      {/* Transcription display */}
      <div className="flex-1 bg-[var(--color-bg-secondary)] rounded-lg border border-[var(--color-border)] overflow-auto">
        <div className="p-4 space-y-4">
          {segments.length === 0 && !pendingText ? (
            <p className="text-[var(--color-text-muted)] text-center py-8">
              Appuyez sur le bouton pour commencer la dictee...
            </p>
          ) : (
            <>
              {segments.map((segment) => (
                <div key={segment.id} className="space-y-1">
                  <div className="flex items-start gap-3">
                    <span className="text-xs text-[var(--color-text-muted)] font-mono mt-1">
                      [{formatTimestamp(segment.startMs)}]
                    </span>
                    <p className="flex-1 text-[var(--color-text-primary)]">
                      {segment.text}
                    </p>
                  </div>
                  <div className="pl-16">
                    <ConfidenceIndicator confidence={segment.confidence} />
                  </div>
                </div>
              ))}

              {pendingText && (
                <div className="flex items-start gap-3 opacity-60">
                  <span className="text-xs text-[var(--color-text-muted)] font-mono mt-1">
                    [--:--]
                  </span>
                  <p className="flex-1 text-[var(--color-text-primary)] italic">
                    {pendingText}
                    <span className="animate-pulse">|</span>
                  </p>
                </div>
              )}
            </>
          )}
        </div>
      </div>

      {/* Action buttons */}
      <div className="flex items-center justify-end gap-3">
        <button
          onClick={handleCopy}
          disabled={!fullText}
          className="flex items-center gap-2 px-4 py-2 rounded-lg bg-[var(--color-bg-tertiary)] hover:bg-[var(--color-border)] disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
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
    </div>
  );
}
