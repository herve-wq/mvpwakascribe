import { useAudioDevices } from "../../hooks/useAudioDevices";
import { useAppStore } from "../../stores/appStore";

export function AudioSettings() {
  const { devices, selectedDeviceId, selectDevice } = useAudioDevices();
  const { audioLevel } = useAppStore();

  return (
    <div className="space-y-4">
      <h3 className="text-sm font-medium text-[var(--color-text-primary)] flex items-center gap-2">
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
        Entree audio
      </h3>

      <div className="space-y-3">
        <div>
          <label className="text-xs text-[var(--color-text-muted)] block mb-1">
            Microphone
          </label>
          <select
            value={selectedDeviceId || ""}
            onChange={(e) => selectDevice(e.target.value)}
            className="w-full px-3 py-2 bg-[var(--color-bg-tertiary)] border border-[var(--color-border)] rounded-lg text-sm text-[var(--color-text-primary)] focus:outline-none focus:border-[var(--color-accent)]"
          >
            {devices.map((device) => (
              <option key={device.id} value={device.id}>
                {device.name}
              </option>
            ))}
          </select>
        </div>

        <div>
          <label className="text-xs text-[var(--color-text-muted)] block mb-1">
            Niveau
          </label>
          <div className="h-2 bg-[var(--color-bg-tertiary)] rounded-full overflow-hidden">
            <div
              className="h-full bg-[var(--color-accent)] transition-all duration-100"
              style={{ width: `${audioLevel * 100}%` }}
            />
          </div>
        </div>
      </div>
    </div>
  );
}
