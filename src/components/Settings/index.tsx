import { AudioSettings } from "./AudioSettings";
import { ShortcutSettings } from "./ShortcutSettings";
import { AppearanceSettings } from "./AppearanceSettings";
// Test button - commenter pour désactiver
import { TestButton } from "../TestButton";

interface SettingsProps {
  onClose: () => void;
}

export function Settings({ onClose }: SettingsProps) {
  return (
    <div className="h-full flex flex-col">
      {/* Header */}
      <div className="p-4 border-b border-[var(--color-border)] flex items-center justify-between">
        <h2 className="font-semibold text-[var(--color-text-primary)]">Parametres</h2>
        <button
          onClick={onClose}
          className="p-1 rounded hover:bg-[var(--color-bg-tertiary)]"
        >
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
              d="M6 18L18 6M6 6l12 12"
            />
          </svg>
        </button>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-auto p-4 space-y-6">
        <AudioSettings />
        <div className="border-t border-[var(--color-border)]" />
        <AppearanceSettings />
        <div className="border-t border-[var(--color-border)]" />
        <ShortcutSettings />

        {/* Model info */}
        <div className="border-t border-[var(--color-border)]" />
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
                d="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z"
              />
            </svg>
            Modele
          </h3>

          <div className="bg-[var(--color-bg-tertiary)] rounded-lg p-3 space-y-2 text-sm">
            <div className="flex justify-between">
              <span className="text-[var(--color-text-muted)]">Moteur</span>
              <span className="text-[var(--color-text-primary)]">
                Parakeet TDT v3 (OpenVINO)
              </span>
            </div>
            <div className="flex justify-between">
              <span className="text-[var(--color-text-muted)]">Statut</span>
              <span className="text-green-500 flex items-center gap-1">
                <span className="w-2 h-2 bg-green-500 rounded-full" />
                Charge
              </span>
            </div>
            <div className="flex justify-between">
              <span className="text-[var(--color-text-muted)]">Device</span>
              <span className="text-[var(--color-text-primary)]">Intel UHD 630</span>
            </div>
          </div>

          {/* Test button - commenter pour désactiver */}
          <TestButton className="mt-4" />
        </div>
      </div>

      {/* Footer */}
      <div className="p-4 border-t border-[var(--color-border)] text-center">
        <p className="text-xs text-[var(--color-text-muted)]">
          WakaScribe v0.1.0 - 100% Offline
        </p>
      </div>
    </div>
  );
}
