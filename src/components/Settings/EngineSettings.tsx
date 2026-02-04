import { useState } from "react";
import { useAppStore } from "../../stores/appStore";
import { ENGINE_BACKENDS, EngineBackend } from "../../lib/types";
import { updateSettings as saveSettings, switchEngineBackend } from "../../lib/tauri";

export function EngineSettings() {
  const { settings, setSettings } = useAppStore();
  const [switching, setSwitching] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleBackendChange = async (backend: EngineBackend) => {
    if (backend === settings.engineBackend) return;

    setSwitching(true);
    setError(null);

    try {
      // Switch the backend in the Rust engine
      await switchEngineBackend(backend);

      // Update local and persisted settings
      const newSettings = { ...settings, engineBackend: backend };
      setSettings(newSettings);
      await saveSettings(newSettings);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
      console.error("Failed to switch backend:", e);
    } finally {
      setSwitching(false);
    }
  };

  const currentBackend = ENGINE_BACKENDS.find(b => b.value === settings.engineBackend) || ENGINE_BACKENDS[0];

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
            d="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z"
          />
        </svg>
        Moteur d'inference
      </h3>

      {/* Backend selector */}
      <div className="space-y-3">
        <label className="text-sm text-[var(--color-text-muted)]">
          Backend
        </label>
        <div className="space-y-2">
          {ENGINE_BACKENDS.map((backend) => (
            <label
              key={backend.value}
              className={`flex items-start gap-3 p-3 rounded-lg border transition-colors ${
                switching ? "opacity-50 cursor-wait" : "cursor-pointer"
              } ${
                settings.engineBackend === backend.value
                  ? "border-[var(--color-primary)] bg-[var(--color-primary)]/10"
                  : "border-[var(--color-border)] hover:border-[var(--color-text-muted)]"
              }`}
            >
              <input
                type="radio"
                name="engineBackend"
                value={backend.value}
                checked={settings.engineBackend === backend.value}
                onChange={() => handleBackendChange(backend.value)}
                disabled={switching}
                className="mt-1"
              />
              <div>
                <div className="font-medium text-[var(--color-text-primary)]">
                  {backend.label}
                  {switching && settings.engineBackend !== backend.value && (
                    <span className="ml-2 text-xs text-[var(--color-text-muted)]">
                      Chargement...
                    </span>
                  )}
                </div>
                <div className="text-xs text-[var(--color-text-muted)]">
                  {backend.description}
                </div>
              </div>
            </label>
          ))}
        </div>

        {/* Error message */}
        {error && (
          <div className="p-2 rounded bg-red-500/10 border border-red-500/30 text-red-500 text-xs">
            Erreur: {error}
          </div>
        )}
      </div>

      {/* Current engine info */}
      <div className="bg-[var(--color-bg-tertiary)] rounded-lg p-3 space-y-2 text-sm">
        <div className="flex justify-between">
          <span className="text-[var(--color-text-muted)]">Modele</span>
          <span className="text-[var(--color-text-primary)]">
            Parakeet TDT v3
          </span>
        </div>
        <div className="flex justify-between">
          <span className="text-[var(--color-text-muted)]">Backend actif</span>
          <span className="text-[var(--color-text-primary)]">
            {currentBackend.label}
          </span>
        </div>
        <div className="flex justify-between">
          <span className="text-[var(--color-text-muted)]">Statut</span>
          {switching ? (
            <span className="text-yellow-500 flex items-center gap-1">
              <span className="w-2 h-2 bg-yellow-500 rounded-full animate-pulse" />
              Chargement...
            </span>
          ) : (
            <span className="text-green-500 flex items-center gap-1">
              <span className="w-2 h-2 bg-green-500 rounded-full" />
              Charge
            </span>
          )}
        </div>
      </div>

      {/* Note */}
      <p className="text-xs text-[var(--color-text-muted)] italic">
        Le changement de backend charge le nouveau modele a chaud.
      </p>
    </div>
  );
}
