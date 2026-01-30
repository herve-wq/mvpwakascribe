use crate::audio::AudioCapture;
use crate::error::Result;
use crate::storage::AudioDevice;
use tauri::State;

pub struct AudioState(pub AudioCapture);

#[tauri::command]
pub fn list_audio_devices() -> Result<Vec<AudioDevice>> {
    AudioCapture::list_devices()
}

#[tauri::command]
pub fn start_recording(
    state: State<'_, AudioState>,
    device_id: Option<String>,
) -> Result<()> {
    state.0.start(device_id.as_deref())
}

#[tauri::command]
pub fn pause_recording(state: State<'_, AudioState>) -> Result<()> {
    state.0.pause()
}

#[tauri::command]
pub fn resume_recording(state: State<'_, AudioState>) -> Result<()> {
    state.0.resume()
}

#[tauri::command]
pub fn get_audio_level(state: State<'_, AudioState>) -> f32 {
    state.0.get_audio_level()
}
