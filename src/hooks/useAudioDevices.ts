import { useEffect } from "react";
import { useAppStore } from "../stores/appStore";
import { listAudioDevices } from "../lib/tauri";

export function useAudioDevices() {
  const { audioDevices, selectedDeviceId, setAudioDevices, setSelectedDeviceId } =
    useAppStore();

  useEffect(() => {
    async function loadDevices() {
      try {
        const devices = await listAudioDevices();
        setAudioDevices(devices);

        // Select default device if none selected
        if (!selectedDeviceId && devices.length > 0) {
          const defaultDevice = devices.find((d) => d.isDefault) || devices[0];
          setSelectedDeviceId(defaultDevice.id);
        }
      } catch (error) {
        console.error("Failed to load audio devices:", error);
      }
    }

    loadDevices();
  }, [selectedDeviceId, setAudioDevices, setSelectedDeviceId]);

  return {
    devices: audioDevices,
    selectedDeviceId,
    selectDevice: setSelectedDeviceId,
  };
}
