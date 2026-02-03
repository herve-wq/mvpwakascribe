/**
 * Composant de test de transcription
 *
 * Affiche un bouton pour tester la transcription avec un fichier audio de référence.
 * Les résultats et métriques sont affichés dans un panneau dédié.
 *
 * Pour désactiver ce composant:
 * 1. Supprimer l'import et l'utilisation dans Layout.tsx ou le composant parent
 * 2. Ou simplement ne pas rendre ce composant
 */

import { useState, useEffect } from "react";
import {
  testTranscription,
  checkTestAudio,
  type TestTranscriptionResult,
} from "../../lib/tauri";

interface TestButtonProps {
  className?: string;
}

export function TestButton({ className = "" }: TestButtonProps) {
  const [isLoading, setIsLoading] = useState(false);
  const [result, setResult] = useState<TestTranscriptionResult | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [hasTestFile, setHasTestFile] = useState<boolean | null>(null);

  // Vérifier si le fichier de test existe au montage
  useEffect(() => {
    checkTestAudio()
      .then(() => setHasTestFile(true))
      .catch(() => setHasTestFile(false));
  }, []);

  const handleTest = async () => {
    setIsLoading(true);
    setError(null);
    setResult(null);

    try {
      const res = await testTranscription();
      setResult(res);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsLoading(false);
    }
  };

  // Ne pas afficher si le fichier de test n'existe pas
  if (hasTestFile === false) {
    return (
      <div className={`p-3 bg-yellow-500/10 border border-yellow-500/30 rounded-lg text-sm ${className}`}>
        <p className="text-yellow-600 dark:text-yellow-400">
          Fichier test_audio.wav manquant dans model/
        </p>
      </div>
    );
  }

  // Chargement initial
  if (hasTestFile === null) {
    return null;
  }

  return (
    <div className={`space-y-3 ${className}`}>
      {/* Bouton de test */}
      <button
        onClick={handleTest}
        disabled={isLoading}
        className={`
          w-full px-4 py-2 rounded-lg font-medium text-sm transition-all
          ${isLoading
            ? "bg-gray-400 cursor-not-allowed"
            : "bg-blue-600 hover:bg-blue-700 text-white"
          }
        `}
      >
        {isLoading ? (
          <span className="flex items-center justify-center gap-2">
            <svg className="animate-spin h-4 w-4" viewBox="0 0 24 24">
              <circle
                className="opacity-25"
                cx="12"
                cy="12"
                r="10"
                stroke="currentColor"
                strokeWidth="4"
                fill="none"
              />
              <path
                className="opacity-75"
                fill="currentColor"
                d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
              />
            </svg>
            Test en cours...
          </span>
        ) : (
          "Test Transcription"
        )}
      </button>

      {/* Erreur */}
      {error && (
        <div className="p-3 bg-red-500/10 border border-red-500/30 rounded-lg">
          <p className="text-red-600 dark:text-red-400 text-sm">{error}</p>
        </div>
      )}

      {/* Résultats */}
      {result && (
        <div className="p-3 bg-[var(--color-bg-secondary)] border border-[var(--color-border)] rounded-lg space-y-3">
          {/* Texte transcrit */}
          <div>
            <p className="text-xs text-[var(--color-text-muted)] mb-1">Résultat:</p>
            <p className="text-sm text-[var(--color-text-primary)] bg-[var(--color-bg-tertiary)] p-2 rounded">
              {result.text || "(vide)"}
            </p>
          </div>

          {/* Métriques */}
          <div className="grid grid-cols-2 gap-2 text-xs">
            <div className="bg-[var(--color-bg-tertiary)] p-2 rounded">
              <p className="text-[var(--color-text-muted)]">Audio</p>
              <p className="text-[var(--color-text-primary)] font-mono">
                {(result.audio_duration_ms / 1000).toFixed(1)}s
              </p>
            </div>
            <div className="bg-[var(--color-bg-tertiary)] p-2 rounded">
              <p className="text-[var(--color-text-muted)]">Traitement</p>
              <p className="text-[var(--color-text-primary)] font-mono">
                {(result.transcription_time_ms / 1000).toFixed(2)}s
              </p>
            </div>
            <div className="bg-[var(--color-bg-tertiary)] p-2 rounded">
              <p className="text-[var(--color-text-muted)]">Vitesse</p>
              <p className="text-[var(--color-text-primary)] font-mono">
                {(1 / result.realtime_factor).toFixed(1)}x
              </p>
            </div>
            <div className="bg-[var(--color-bg-tertiary)] p-2 rounded">
              <p className="text-[var(--color-text-muted)]">Audio RMS</p>
              <p className="text-[var(--color-text-primary)] font-mono">
                {result.diagnostics.audio_rms.toFixed(4)}
              </p>
            </div>
          </div>

          {/* Fichier source */}
          <p className="text-xs text-[var(--color-text-muted)] truncate">
            {result.audio_file.split("/").pop()}
          </p>
        </div>
      )}
    </div>
  );
}
