import type { DecodingConfig, TranscriptionSettings } from "./types";

/// Build a DecodingConfig from the app's TranscriptionSettings
export function getDecodingConfig(settings: TranscriptionSettings): DecodingConfig {
  return {
    beam_width: settings.beamWidth,
    temperature: settings.temperature,
    blank_penalty: settings.blankPenalty,
  };
}
