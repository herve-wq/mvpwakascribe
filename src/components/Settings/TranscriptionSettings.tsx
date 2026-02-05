import { useAppStore } from "../../stores/appStore";
import { TRANSCRIPTION_LANGUAGES } from "../../lib/types";
import type { TranscriptionLanguage } from "../../lib/types";

export function TranscriptionSettings() {
  const { settings, setSettings } = useAppStore();
  const { transcription, engineBackend } = settings;

  // CoreML doesn't support beam search
  const isCoreML = engineBackend === "coreml";

  const handleLanguageChange = (language: TranscriptionLanguage) => {
    setSettings({
      transcription: { ...transcription, language },
    });
  };

  const handleBeamWidthChange = (beamWidth: number) => {
    // Ignore beam width changes for CoreML (always greedy)
    if (isCoreML) return;
    setSettings({
      transcription: { ...transcription, beamWidth },
    });
  };

  const handleTemperatureChange = (temperature: number) => {
    setSettings({
      transcription: { ...transcription, temperature },
    });
  };

  const handleBlankPenaltyChange = (blankPenalty: number) => {
    setSettings({
      transcription: { ...transcription, blankPenalty },
    });
  };

  // Decode mode: Simple (greedy) vs Precise (beam search)
  // CoreML only supports greedy decoding
  const isBeamSearch = !isCoreML && transcription.beamWidth > 1;

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
        Transcription
      </h3>

      {/* Language selector */}
      <div className="space-y-2">
        <label className="text-sm text-[var(--color-text-secondary)]">
          Langue
        </label>
        <select
          value={transcription.language}
          onChange={(e) => handleLanguageChange(e.target.value as TranscriptionLanguage)}
          className="w-full px-3 py-2 rounded-lg bg-[var(--color-bg-tertiary)] border border-[var(--color-border)]
                     text-[var(--color-text-primary)] text-sm
                     focus:outline-none focus:ring-2 focus:ring-[var(--color-accent)] focus:border-transparent"
        >
          {TRANSCRIPTION_LANGUAGES.map((lang) => (
            <option key={lang.value} value={lang.value}>
              {lang.label}
            </option>
          ))}
        </select>
      </div>

      {/* Decoding mode */}
      <div className="space-y-2">
        <label className="text-sm text-[var(--color-text-secondary)]">
          Mode de decodage
        </label>
        <div className="flex gap-2">
          <button
            onClick={() => handleBeamWidthChange(1)}
            className={`flex-1 px-3 py-2 rounded-lg text-sm font-medium transition-colors
              ${!isBeamSearch
                ? "bg-[var(--color-accent)] text-white"
                : "bg-[var(--color-bg-tertiary)] text-[var(--color-text-secondary)] hover:bg-[var(--color-border)]"
              }`}
          >
            Rapide
          </button>
          <button
            onClick={() => handleBeamWidthChange(5)}
            className={`flex-1 px-3 py-2 rounded-lg text-sm font-medium transition-colors
              ${isBeamSearch
                ? "bg-[var(--color-accent)] text-white"
                : "bg-[var(--color-bg-tertiary)] text-[var(--color-text-secondary)] hover:bg-[var(--color-border)]"
              }`}
          >
            Precis
          </button>
        </div>
        <p className="text-xs text-[var(--color-text-muted)]">
          {isCoreML
            ? "CoreML: Greedy uniquement (beam search non supporte)"
            : isBeamSearch
              ? "Beam search (beam=5): Plus lent mais meilleure qualite"
              : "Greedy (beam=1): Rapide, bonne qualite"}
        </p>
      </div>

      {/* Advanced: Beam width slider (only shown in precise mode) */}
      {isBeamSearch && (
        <div className="space-y-2">
          <div className="flex justify-between">
            <label className="text-sm text-[var(--color-text-secondary)]">
              Beam width
            </label>
            <span className="text-sm text-[var(--color-text-muted)]">
              {transcription.beamWidth}
            </span>
          </div>
          <input
            type="range"
            min="2"
            max="10"
            step="1"
            value={transcription.beamWidth}
            onChange={(e) => handleBeamWidthChange(parseInt(e.target.value))}
            className="w-full accent-[var(--color-accent)]"
          />
          <div className="flex justify-between text-xs text-[var(--color-text-muted)]">
            <span>2</span>
            <span>10</span>
          </div>
        </div>
      )}

      {/* Temperature slider */}
      <div className="space-y-2">
        <div className="flex justify-between">
          <label className="text-sm text-[var(--color-text-secondary)]">
            Temperature
          </label>
          <span className="text-sm text-[var(--color-text-muted)]">
            {transcription.temperature.toFixed(1)}
          </span>
        </div>
        <input
          type="range"
          min="0.1"
          max="1.5"
          step="0.1"
          value={transcription.temperature}
          onChange={(e) => handleTemperatureChange(parseFloat(e.target.value))}
          className="w-full accent-[var(--color-accent)]"
        />
        <div className="flex justify-between text-xs text-[var(--color-text-muted)]">
          <span>Conservateur</span>
          <span>Creatif</span>
        </div>
      </div>

      {/* Blank penalty slider */}
      <div className="space-y-2">
        <div className="flex justify-between">
          <label className="text-sm text-[var(--color-text-secondary)]">
            Blank Penalty
          </label>
          <span className="text-sm text-[var(--color-text-muted)]">
            {transcription.blankPenalty.toFixed(1)}
          </span>
        </div>
        <input
          type="range"
          min="0"
          max="15"
          step="0.5"
          value={transcription.blankPenalty}
          onChange={(e) => handleBlankPenaltyChange(parseFloat(e.target.value))}
          className="w-full accent-[var(--color-accent)]"
        />
        <div className="flex justify-between text-xs text-[var(--color-text-muted)]">
          <span>Plus de blanks</span>
          <span>Plus de tokens</span>
        </div>
      </div>

      {/* Current config summary */}
      <div className="bg-[var(--color-bg-tertiary)] rounded-lg p-3 space-y-1 text-xs">
        <div className="flex justify-between">
          <span className="text-[var(--color-text-muted)]">Config actuelle</span>
          <span className="text-[var(--color-text-primary)] font-mono">
            beam={isCoreML ? 1 : transcription.beamWidth}, temp={transcription.temperature.toFixed(1)}, blank={transcription.blankPenalty.toFixed(1)}
          </span>
        </div>
      </div>
    </div>
  );
}
