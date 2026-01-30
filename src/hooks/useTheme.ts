import { useEffect, useState } from "react";
import { useAppStore } from "../stores/appStore";

type Theme = "light" | "dark";

export function useTheme() {
  const { settings, setSettings } = useAppStore();
  const [resolvedTheme, setResolvedTheme] = useState<Theme>("light");

  useEffect(() => {
    function updateTheme() {
      let theme: Theme;

      if (settings.theme === "system") {
        theme = window.matchMedia("(prefers-color-scheme: dark)").matches
          ? "dark"
          : "light";
      } else {
        theme = settings.theme;
      }

      setResolvedTheme(theme);

      // Update document class for CSS
      document.documentElement.classList.remove("light", "dark");
      document.documentElement.classList.add(theme);
    }

    updateTheme();

    // Listen for system theme changes
    const mediaQuery = window.matchMedia("(prefers-color-scheme: dark)");
    mediaQuery.addEventListener("change", updateTheme);

    return () => {
      mediaQuery.removeEventListener("change", updateTheme);
    };
  }, [settings.theme]);

  const setTheme = (theme: "light" | "dark" | "system") => {
    setSettings({ theme });
  };

  const toggleTheme = () => {
    const newTheme = resolvedTheme === "light" ? "dark" : "light";
    setTheme(newTheme);
  };

  return {
    theme: settings.theme,
    resolvedTheme,
    setTheme,
    toggleTheme,
  };
}
