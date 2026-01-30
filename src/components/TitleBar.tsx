import { getCurrentWindow } from "@tauri-apps/api/window";
import { useTheme } from "../hooks/useTheme";

export function TitleBar() {
  const { resolvedTheme, toggleTheme } = useTheme();
  const appWindow = getCurrentWindow();

  const handleMinimize = () => appWindow.minimize();
  const handleMaximize = () => appWindow.toggleMaximize();
  const handleClose = () => appWindow.close();

  return (
    <div className="titlebar-drag h-[38px] bg-[var(--color-bg-secondary)] border-b border-[var(--color-border)] flex items-center justify-between px-4 select-none">
      {/* macOS traffic lights spacing */}
      <div className="w-20" />

      {/* Title */}
      <div className="font-semibold text-sm text-[var(--color-text-primary)]">
        WakaScribe
      </div>

      {/* Controls */}
      <div className="titlebar-no-drag flex items-center gap-2">
        {/* Theme toggle */}
        <button
          onClick={toggleTheme}
          className="p-1.5 rounded hover:bg-[var(--color-bg-tertiary)] transition-colors"
          title={resolvedTheme === "light" ? "Dark mode" : "Light mode"}
        >
          {resolvedTheme === "light" ? (
            <svg
              className="w-4 h-4 text-[var(--color-text-secondary)]"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M20.354 15.354A9 9 0 018.646 3.646 9.003 9.003 0 0012 21a9.003 9.003 0 008.354-5.646z"
              />
            </svg>
          ) : (
            <svg
              className="w-4 h-4 text-[var(--color-text-secondary)]"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M12 3v1m0 16v1m9-9h-1M4 12H3m15.364 6.364l-.707-.707M6.343 6.343l-.707-.707m12.728 0l-.707.707M6.343 17.657l-.707.707M16 12a4 4 0 11-8 0 4 4 0 018 0z"
              />
            </svg>
          )}
        </button>

        {/* Window controls - hidden on macOS as we use native traffic lights */}
        <div className="hidden">
          <button onClick={handleMinimize} className="p-1">
            <svg className="w-4 h-4" viewBox="0 0 24 24">
              <path fill="currentColor" d="M19 13H5v-2h14v2z" />
            </svg>
          </button>
          <button onClick={handleMaximize} className="p-1">
            <svg className="w-4 h-4" viewBox="0 0 24 24">
              <path fill="currentColor" d="M4 4h16v16H4V4zm2 2v12h12V6H6z" />
            </svg>
          </button>
          <button onClick={handleClose} className="p-1">
            <svg className="w-4 h-4" viewBox="0 0 24 24">
              <path
                fill="currentColor"
                d="M19 6.41L17.59 5 12 10.59 6.41 5 5 6.41 10.59 12 5 17.59 6.41 19 12 13.41 17.59 19 19 17.59 13.41 12 19 6.41z"
              />
            </svg>
          </button>
        </div>
      </div>
    </div>
  );
}
