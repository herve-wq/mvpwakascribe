import { AudioSettings } from "./AudioSettings";
import { ShortcutSettings } from "./ShortcutSettings";
import { AppearanceSettings } from "./AppearanceSettings";
import { TranscriptionSettings } from "./TranscriptionSettings";
import { EngineSettings } from "./EngineSettings";
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
        <TranscriptionSettings />
        <div className="border-t border-[var(--color-border)]" />
        <AudioSettings />
        <div className="border-t border-[var(--color-border)]" />
        <AppearanceSettings />
        <div className="border-t border-[var(--color-border)]" />
        <ShortcutSettings />

        {/* Engine settings */}
        <div className="border-t border-[var(--color-border)]" />
        <EngineSettings />

        {/* Test button - commenter pour désactiver */}
        <div className="border-t border-[var(--color-border)]" />
        <TestButton className="mt-4" />
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
