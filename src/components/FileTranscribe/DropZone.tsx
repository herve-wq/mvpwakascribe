import { useState, useCallback } from "react";
import { open } from "@tauri-apps/plugin-dialog";

interface DropZoneProps {
  onFileSelect: (path: string) => void;
  disabled?: boolean;
}

export function DropZone({ onFileSelect, disabled }: DropZoneProps) {
  const [isDragging, setIsDragging] = useState(false);

  const handleDragOver = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    setIsDragging(true);
  }, []);

  const handleDragLeave = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    setIsDragging(false);
  }, []);

  const handleDrop = useCallback(
    (e: React.DragEvent) => {
      e.preventDefault();
      setIsDragging(false);

      const files = Array.from(e.dataTransfer.files);
      if (files.length > 0) {
        // In Tauri, we'd need to handle this differently
        // For now, prompt user to use the file picker
        handleFileSelect();
      }
    },
    [onFileSelect]
  );

  const handleFileSelect = async () => {
    if (disabled) return;

    try {
      const selected = await open({
        multiple: false,
        filters: [
          {
            name: "Audio",
            extensions: ["wav", "mp3", "m4a", "ogg", "flac"],
          },
        ],
      });

      if (selected && typeof selected === "string") {
        onFileSelect(selected);
      }
    } catch (error) {
      console.error("Failed to open file dialog:", error);
    }
  };

  return (
    <div
      onDragOver={handleDragOver}
      onDragLeave={handleDragLeave}
      onDrop={handleDrop}
      onClick={handleFileSelect}
      className={`
        border-2 border-dashed rounded-xl p-12 text-center cursor-pointer transition-all
        ${disabled ? "opacity-50 cursor-not-allowed" : ""}
        ${
          isDragging
            ? "border-[var(--color-accent)] bg-[var(--color-accent)]/5"
            : "border-[var(--color-border)] hover:border-[var(--color-text-muted)] hover:bg-[var(--color-bg-secondary)]"
        }
      `}
    >
      <div className="flex flex-col items-center gap-4">
        <div className="w-16 h-16 rounded-full bg-[var(--color-bg-tertiary)] flex items-center justify-center">
          <svg
            className="w-8 h-8 text-[var(--color-text-muted)]"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12"
            />
          </svg>
        </div>
        <div>
          <p className="text-[var(--color-text-primary)] font-medium">
            Glissez un fichier audio ici
          </p>
          <p className="text-sm text-[var(--color-text-muted)] mt-1">
            ou cliquez pour selectionner
          </p>
        </div>
        <p className="text-xs text-[var(--color-text-muted)]">
          Formats: .wav .mp3 .m4a .ogg .flac
        </p>
      </div>
    </div>
  );
}
