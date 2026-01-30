import { useState, useRef, useEffect } from "react";
import { save } from "@tauri-apps/plugin-dialog";

interface ExportMenuProps {
  onExportTxt: (path: string) => void;
  onExportDocx: (path: string) => void;
  onCopy: () => void;
  disabled?: boolean;
}

export function ExportMenu({
  onExportTxt,
  onExportDocx,
  onCopy,
  disabled,
}: ExportMenuProps) {
  const [isOpen, setIsOpen] = useState(false);
  const menuRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    function handleClickOutside(event: MouseEvent) {
      if (menuRef.current && !menuRef.current.contains(event.target as Node)) {
        setIsOpen(false);
      }
    }

    document.addEventListener("mousedown", handleClickOutside);
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, []);

  const handleExportTxt = async () => {
    setIsOpen(false);
    const path = await save({
      filters: [{ name: "Text", extensions: ["txt"] }],
      defaultPath: "transcription.txt",
    });
    if (path) {
      onExportTxt(path);
    }
  };

  const handleExportDocx = async () => {
    setIsOpen(false);
    const path = await save({
      filters: [{ name: "Word Document", extensions: ["docx"] }],
      defaultPath: "transcription.docx",
    });
    if (path) {
      onExportDocx(path);
    }
  };

  const handleCopy = () => {
    setIsOpen(false);
    onCopy();
  };

  return (
    <div ref={menuRef} className="relative">
      <button
        onClick={() => setIsOpen(!isOpen)}
        disabled={disabled}
        className="flex items-center gap-2 px-4 py-2 rounded-lg bg-[var(--color-accent)] text-white hover:bg-[var(--color-accent-hover)] disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
      >
        <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path
            strokeLinecap="round"
            strokeLinejoin="round"
            strokeWidth={2}
            d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-8l-4-4m0 0L8 8m4-4v12"
          />
        </svg>
        <span className="text-sm">Exporter</span>
        <svg className="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path
            strokeLinecap="round"
            strokeLinejoin="round"
            strokeWidth={2}
            d="M19 9l-7 7-7-7"
          />
        </svg>
      </button>

      {isOpen && (
        <div className="absolute right-0 mt-2 w-48 bg-[var(--color-bg-primary)] border border-[var(--color-border)] rounded-lg shadow-lg overflow-hidden z-10">
          <button
            onClick={handleCopy}
            className="w-full px-4 py-2 text-left text-sm hover:bg-[var(--color-bg-secondary)] flex items-center gap-2"
          >
            <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z"
              />
            </svg>
            Copier dans le presse-papier
          </button>
          <button
            onClick={handleExportTxt}
            className="w-full px-4 py-2 text-left text-sm hover:bg-[var(--color-bg-secondary)] flex items-center gap-2"
          >
            <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
              />
            </svg>
            Exporter en .txt
          </button>
          <button
            onClick={handleExportDocx}
            className="w-full px-4 py-2 text-left text-sm hover:bg-[var(--color-bg-secondary)] flex items-center gap-2"
          >
            <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M7 21h10a2 2 0 002-2V9.414a1 1 0 00-.293-.707l-5.414-5.414A1 1 0 0012.586 3H7a2 2 0 00-2 2v14a2 2 0 002 2z"
              />
            </svg>
            Exporter en .docx
          </button>
        </div>
      )}
    </div>
  );
}
