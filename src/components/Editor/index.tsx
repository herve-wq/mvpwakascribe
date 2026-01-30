import { useState, useEffect } from "react";
import { SegmentList } from "./SegmentList";
import { ExportMenu } from "./ExportMenu";
import { useTranscription } from "../../hooks/useTranscription";
import type { Transcription } from "../../lib/types";

interface EditorProps {
  transcription: Transcription;
  onClose?: () => void;
}

export function Editor({ transcription, onClose }: EditorProps) {
  const [editedText, setEditedText] = useState(
    transcription.editedText || transcription.rawText
  );
  const [showSegments, setShowSegments] = useState(false);
  const { updateText, exportTxt, exportDocx, copyText } = useTranscription();

  useEffect(() => {
    setEditedText(transcription.editedText || transcription.rawText);
  }, [transcription]);

  const handleSave = async () => {
    await updateText(transcription.id, editedText);
  };

  const handleExportTxt = (path: string) => {
    exportTxt(transcription.id, path);
  };

  const handleExportDocx = (path: string) => {
    exportDocx(transcription.id, path);
  };

  const handleCopy = () => {
    copyText(editedText);
  };

  const hasChanges = editedText !== (transcription.editedText || transcription.rawText);

  return (
    <div className="h-full flex flex-col">
      {/* Header */}
      <div className="p-4 border-b border-[var(--color-border)] flex items-center justify-between">
        <div>
          <h2 className="font-semibold text-[var(--color-text-primary)]">
            {transcription.sourceName || "Dictee"}
          </h2>
          <p className="text-xs text-[var(--color-text-muted)]">
            {new Date(transcription.createdAt).toLocaleString("fr-FR")}
          </p>
        </div>
        <div className="flex items-center gap-2">
          <button
            onClick={() => setShowSegments(!showSegments)}
            className={`px-3 py-1.5 rounded text-sm transition-colors ${
              showSegments
                ? "bg-[var(--color-accent)] text-white"
                : "bg-[var(--color-bg-tertiary)] hover:bg-[var(--color-border)]"
            }`}
          >
            Segments
          </button>
          {onClose && (
            <button
              onClick={onClose}
              className="p-1.5 rounded hover:bg-[var(--color-bg-tertiary)]"
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
          )}
        </div>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-auto p-4">
        {showSegments ? (
          <SegmentList segments={transcription.segments} />
        ) : (
          <textarea
            value={editedText}
            onChange={(e) => setEditedText(e.target.value)}
            className="w-full h-full resize-none bg-transparent text-[var(--color-text-primary)] focus:outline-none"
            placeholder="Transcription vide..."
          />
        )}
      </div>

      {/* Footer */}
      <div className="p-4 border-t border-[var(--color-border)] flex items-center justify-between">
        <div>
          {hasChanges && (
            <span className="text-xs text-[var(--color-warning)]">
              Modifications non enregistrees
            </span>
          )}
        </div>
        <div className="flex items-center gap-3">
          {hasChanges && (
            <button
              onClick={handleSave}
              className="px-4 py-2 rounded-lg bg-[var(--color-bg-tertiary)] hover:bg-[var(--color-border)] text-sm transition-colors"
            >
              Enregistrer
            </button>
          )}
          <ExportMenu
            onExportTxt={handleExportTxt}
            onExportDocx={handleExportDocx}
            onCopy={handleCopy}
          />
        </div>
      </div>
    </div>
  );
}
