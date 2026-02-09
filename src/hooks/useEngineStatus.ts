import { useEffect } from "react";
import { useAppStore } from "../stores/appStore";
import { getEngineStatus } from "../lib/tauri";
import { ENGINE_BACKENDS } from "../lib/types";

export function useEngineStatus() {
  const engineStatus = useAppStore((s) => s.engineStatus);
  const setEngineStatus = useAppStore((s) => s.setEngineStatus);
  const settings = useAppStore((s) => s.settings);
  const setSettings = useAppStore((s) => s.setSettings);

  useEffect(() => {
    getEngineStatus()
      .then((status) => {
        setEngineStatus(status);
        // Sync settings.engineBackend with the actual backend if they differ
        // (can happen after a fallback at startup)
        if (status.backend) {
          const actualValue = ENGINE_BACKENDS.find(b => b.label === status.backend)?.value;
          if (actualValue && actualValue !== settings.engineBackend) {
            setSettings({ engineBackend: actualValue });
          }
        }
      })
      .catch((e) => {
        console.error("Failed to get engine status:", e);
        setEngineStatus({ backend: "", isLoaded: false, error: String(e) });
      });
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return engineStatus;
}
