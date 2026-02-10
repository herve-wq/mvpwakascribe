import { RangeSlider } from "../ui/RangeSlider";
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
            className={!isBeamSearch ? "btn-toggle-active" : "btn-toggle"}
          >
            Rapide
          </button>
          <button
            onClick={() => handleBeamWidthChange(5)}
            className={isBeamSearch ? "btn-toggle-active" : "btn-toggle"}
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
        <RangeSlider
          label="Beam width"
          value={transcription.beamWidth}
          min={2}
          max={10}
          step={1}
          onChange={handleBeamWidthChange}
          minLabel="2"
          maxLabel="10"
        />
      )}

      {/* Temperature slider */}
      <RangeSlider
        label="Temperature"
        value={transcription.temperature}
        min={0.1}
        max={1.5}
        step={0.1}
        onChange={handleTemperatureChange}
        formatValue={(v) => v.toFixed(1)}
        minLabel="Conservateur"
        maxLabel="Creatif"
      />

      {/* Blank penalty slider */}
      <RangeSlider
        label="Blank Penalty"
        value={transcription.blankPenalty}
        min={0}
        max={15}
        step={0.5}
        onChange={handleBlankPenaltyChange}
        formatValue={(v) => v.toFixed(1)}
        minLabel="Plus de blanks"
        maxLabel="Plus de tokens"
      />

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
