import { useState } from "react";
import { Layout } from "./components/Layout";
import { Recorder } from "./components/Recorder";
import { FileTranscribe } from "./components/FileTranscribe";
import type { TranscriptionMode } from "./lib/types";

function App() {
  const [mode, setMode] = useState<TranscriptionMode>("dictation");

  return (
    <Layout>
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
