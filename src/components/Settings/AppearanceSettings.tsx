import { useTheme } from "../../hooks/useTheme";

export function AppearanceSettings() {
  const { theme, setTheme } = useTheme();

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
            d="M7 21a4 4 0 01-4-4V5a2 2 0 012-2h4a2 2 0 012 2v12a4 4 0 01-4 4zm0 0h12a2 2 0 002-2v-4a2 2 0 00-2-2h-2.343M11 7.343l1.657-1.657a2 2 0 012.828 0l2.829 2.829a2 2 0 010 2.828l-8.486 8.485M7 17h.01"
          />
        </svg>
        Apparence
      </h3>

      <div className="space-y-2">
        <label className="text-xs text-[var(--color-text-muted)] block">Theme</label>
        <div className="flex gap-2">
          <button
            onClick={() => setTheme("light")}
            className={theme === "light" ? "btn-toggle-active" : "btn-toggle"}
          >
            Clair
          </button>
          <button
            onClick={() => setTheme("dark")}
            className={theme === "dark" ? "btn-toggle-active" : "btn-toggle"}
          >
            Sombre
          </button>
          <button
            onClick={() => setTheme("system")}
            className={theme === "system" ? "btn-toggle-active" : "btn-toggle"}
          >
            Systeme
          </button>
        </div>
      </div>
    </div>
  );
}
