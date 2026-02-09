import { useState } from "react";
import { Layout } from "./components/Layout";
import { Recorder } from "./components/Recorder";
import { FileTranscribe } from "./components/FileTranscribe";
import { useEngineStatus } from "./hooks/useEngineStatus";
import type { TranscriptionMode } from "./lib/types";

function App() {
  const [mode, setMode] = useState<TranscriptionMode>("dictation");
  const engineStatus = useEngineStatus();

  return (
    <Layout>
      {/* Engine not loaded warning */}
      {!engineStatus.isLoaded && (
        <div className="px-4 py-2 bg-red-500/10 border-b border-red-500/30 text-red-600 dark:text-red-400 text-sm flex items-center gap-2">
          <svg className="w-4 h-4 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L4.082 16.5c-.77.833.192 2.5 1.732 2.5z" />
          </svg>
          <span>
            Moteur non charge.{" "}
            {engineStatus.error ? engineStatus.error : "Verifiez les parametres du moteur."}
          </span>
        </div>
      )}

      {/* Mode tabs */}
      <div className="border-b border-[var(--color-border)]">
        <div className="flex">
          <button
            onClick={() => setMode("dictation")}
            className={`px-6 py-3 text-sm font-medium transition-colors relative ${
              mode === "dictation"
                ? "text-[var(--color-accent)]"
                : "text-[var(--color-text-muted)] hover:text-[var(--color-text-primary)]"
            }`}
          >
            Dictee
            {mode === "dictation" && (
              <span className="absolute bottom-0 left-0 right-0 h-0.5 bg-[var(--color-accent)]" />
            )}
          </button>
          <button
            onClick={() => setMode("file")}
            className={`px-6 py-3 text-sm font-medium transition-colors relative ${
              mode === "file"
                ? "text-[var(--color-accent)]"
                : "text-[var(--color-text-muted)] hover:text-[var(--color-text-primary)]"
            }`}
          >
            Fichier
            {mode === "file" && (
              <span className="absolute bottom-0 left-0 right-0 h-0.5 bg-[var(--color-accent)]" />
            )}
          </button>
        </div>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-hidden">
        {mode === "dictation" ? <Recorder /> : <FileTranscribe />}
      </div>
    </Layout>
  );
}

export default App;
