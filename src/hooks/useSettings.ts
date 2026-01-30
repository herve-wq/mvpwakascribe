import { useEffect } from "react";
import { useAppStore } from "../stores/appStore";
import { getSettings, updateSettings as tauriUpdateSettings } from "../lib/tauri";
import type { Settings } from "../lib/types";

export function useSettings() {
  const { settings, setSettings } = useAppStore();

  // Load settings from backend on mount
  useEffect(() => {
    async function loadSettings() {
      try {
        const savedSettings = await getSettings();
        setSettings(savedSettings);
      } catch (error) {
        console.error("Failed to load settings:", error);
      }
    }

    loadSettings();
  }, [setSettings]);

  const updateSettings = async (newSettings: Partial<Settings>) => {
    try {
      await tauriUpdateSettings(newSettings);
      setSettings(newSettings);
    } catch (error) {
      console.error("Failed to save settings:", error);
    }
  };

  return {
    settings,
    updateSettings,
  };
}
