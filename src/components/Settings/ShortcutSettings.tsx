import { useSettings } from "../../hooks/useSettings";

interface ShortcutItemProps {
  label: string;
  shortcut: string;
}

function ShortcutItem({ label, shortcut }: ShortcutItemProps) {
  // Convert shortcut to display format
  const displayShortcut = shortcut
    .replace("CommandOrControl", "Cmd")
    .replace("+", " + ");

  return (
    <div className="flex items-center justify-between py-2">
      <span className="text-sm text-[var(--color-text-secondary)]">{label}</span>
      <kbd className="px-2 py-1 bg-[var(--color-bg-tertiary)] rounded text-xs font-mono text-[var(--color-text-primary)]">
        {displayShortcut}
      </kbd>
    </div>
  );
}

export function ShortcutSettings() {
  const { settings } = useSettings();

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
            d="M12 19l9 2-9-18-9 18 9-2zm0 0v-8"
          />
        </svg>
        Raccourcis globaux
      </h3>

      <div className="divide-y divide-[var(--color-border)]">
        <ShortcutItem
          label="Demarrer/Arreter dictee"
          shortcut={settings.shortcuts.toggleRecording}
        />
        <ShortcutItem label="Pause" shortcut={settings.shortcuts.pause} />
        <ShortcutItem
          label="Copier transcription"
          shortcut={settings.shortcuts.copy}
        />
      </div>

      <p className="text-xs text-[var(--color-text-muted)]">
        Les raccourcis fonctionnent depuis n'importe quelle application.
      </p>
    </div>
  );
}
