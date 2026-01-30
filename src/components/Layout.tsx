import { ReactNode } from "react";
import { TitleBar } from "./TitleBar";
import { useAppStore } from "../stores/appStore";
import { Settings } from "./Settings";
import { History } from "./History";

interface LayoutProps {
  children: ReactNode;
}

export function Layout({ children }: LayoutProps) {
  const { showSettings, showHistory, toggleSettings, toggleHistory } =
    useAppStore();

  return (
    <div className="h-screen flex flex-col bg-[var(--color-bg-primary)]">
      <TitleBar />

      <div className="flex-1 flex overflow-hidden">
        {/* Main content */}
        <main className="flex-1 overflow-auto">{children}</main>

        {/* Sidebar panels */}
        {showHistory && (
          <aside className="w-80 border-l border-[var(--color-border)] bg-[var(--color-bg-secondary)] overflow-auto">
            <History onClose={toggleHistory} />
          </aside>
        )}

        {showSettings && (
          <aside className="w-80 border-l border-[var(--color-border)] bg-[var(--color-bg-secondary)] overflow-auto">
            <Settings onClose={toggleSettings} />
          </aside>
        )}
      </div>

      {/* Bottom bar with shortcuts hint */}
      <div className="h-8 bg-[var(--color-bg-secondary)] border-t border-[var(--color-border)] flex items-center justify-between px-4">
        <div className="flex items-center gap-4 text-xs text-[var(--color-text-muted)]">
          <span>
            <kbd className="px-1 py-0.5 bg-[var(--color-bg-tertiary)] rounded text-[10px]">
              Cmd+Shift+R
            </kbd>{" "}
            Enregistrer
          </span>
          <span>
            <kbd className="px-1 py-0.5 bg-[var(--color-bg-tertiary)] rounded text-[10px]">
              Cmd+Shift+S
            </kbd>{" "}
            Stop
          </span>
        </div>

        <div className="flex items-center gap-2">
          <button
            onClick={toggleHistory}
            className={`p-1.5 rounded transition-colors ${
              showHistory
                ? "bg-[var(--color-accent)] text-white"
                : "hover:bg-[var(--color-bg-tertiary)] text-[var(--color-text-secondary)]"
            }`}
            title="Historique"
          >
            <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z"
              />
            </svg>
          </button>
          <button
            onClick={toggleSettings}
            className={`p-1.5 rounded transition-colors ${
              showSettings
                ? "bg-[var(--color-accent)] text-white"
                : "hover:bg-[var(--color-bg-tertiary)] text-[var(--color-text-secondary)]"
            }`}
            title="Parametres"
          >
            <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"
              />
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"
              />
            </svg>
          </button>
        </div>
      </div>
    </div>
  );
}
