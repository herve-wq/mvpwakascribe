import { PanelHeader } from "../ui/PanelHeader";
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
      <PanelHeader title="Parametres" onClose={onClose} />

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
