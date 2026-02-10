import { useState, useEffect } from "react";
import { PanelHeader } from "../ui/PanelHeader";
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
      <PanelHeader
        title={transcription.sourceName || "Dictee"}
        subtitle={new Date(transcription.createdAt).toLocaleString("fr-FR")}
        onClose={onClose}
        actions={
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
        }
      />

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
              className="btn-secondary"
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
