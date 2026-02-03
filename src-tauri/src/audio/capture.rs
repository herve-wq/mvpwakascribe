use crate::error::{AppError, Result};
use crate::storage::AudioDevice;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, StreamConfig};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;
use parking_lot::Mutex;
use tracing::{debug, info, warn};

/// Commands that can be sent to the audio thread
enum AudioCommand {
    Start {
        device_id: Option<String>,
        response: Sender<Result<()>>,
    },
    Stop {
        response: Sender<Result<Vec<f32>>>,
    },
    Pause,
    Resume,
    Shutdown,
}

/// Audio capture manager that handles threading internally
pub struct AudioCapture {
    command_tx: Sender<AudioCommand>,
    _thread_handle: JoinHandle<()>,
    is_recording: Arc<AtomicBool>,
    is_paused: Arc<AtomicBool>,
    audio_level: Arc<Mutex<f32>>,
    sample_rate: Arc<Mutex<u32>>,
}

impl AudioCapture {
    pub fn new() -> Self {
        let (command_tx, command_rx) = mpsc::channel();
        let is_recording = Arc::new(AtomicBool::new(false));
        let is_paused = Arc::new(AtomicBool::new(false));
        let audio_level = Arc::new(Mutex::new(0.0f32));
        let sample_rate = Arc::new(Mutex::new(16000u32));

        let is_recording_clone = Arc::clone(&is_recording);
        let is_paused_clone = Arc::clone(&is_paused);
        let audio_level_clone = Arc::clone(&audio_level);
        let sample_rate_clone = Arc::clone(&sample_rate);

        let thread_handle = thread::spawn(move || {
            audio_thread(
                command_rx,
                is_recording_clone,
                is_paused_clone,
                audio_level_clone,
                sample_rate_clone,
            );
        });

        Self {
            command_tx,
            _thread_handle: thread_handle,
            is_recording,
            is_paused,
            audio_level,
            sample_rate,
        }
    }

    pub fn list_devices() -> Result<Vec<AudioDevice>> {
        let host = cpal::default_host();
        let default_device = host.default_input_device();
        let default_name = default_device
            .as_ref()
            .and_then(|d| d.name().ok())
            .unwrap_or_default();

        let devices: Vec<AudioDevice> = host
            .input_devices()
            .map_err(|e| AppError::Audio(e.to_string()))?
            .filter_map(|device| {
                let name = device.name().ok()?;
                Some(AudioDevice {
                    id: name.clone(),
                    name: name.clone(),
                    is_default: name == default_name,
                })
            })
            .collect();

        Ok(devices)
    }

    pub fn start(&self, device_id: Option<&str>) -> Result<()> {
        if self.is_recording.load(Ordering::SeqCst) {
            return Err(AppError::InvalidState("Already recording".into()));
        }

        let (response_tx, response_rx) = mpsc::channel();
        self.command_tx
            .send(AudioCommand::Start {
                device_id: device_id.map(String::from),
                response: response_tx,
            })
            .map_err(|_| AppError::Audio("Audio thread not responding".into()))?;

        response_rx
            .recv()
            .map_err(|_| AppError::Audio("Failed to get response from audio thread".into()))?
    }

    pub fn stop(&self) -> Result<Vec<f32>> {
        if !self.is_recording.load(Ordering::SeqCst) {
            return Err(AppError::InvalidState("Not recording".into()));
        }

        let (response_tx, response_rx) = mpsc::channel();
        self.command_tx
            .send(AudioCommand::Stop {
                response: response_tx,
            })
            .map_err(|_| AppError::Audio("Audio thread not responding".into()))?;

        response_rx
            .recv()
            .map_err(|_| AppError::Audio("Failed to get response from audio thread".into()))?
    }

    pub fn pause(&self) -> Result<()> {
        if !self.is_recording.load(Ordering::SeqCst) {
            return Err(AppError::InvalidState("Not recording".into()));
        }
        self.command_tx
            .send(AudioCommand::Pause)
            .map_err(|_| AppError::Audio("Audio thread not responding".into()))?;
        self.is_paused.store(true, Ordering::SeqCst);
        info!("Recording paused");
        Ok(())
    }

    pub fn resume(&self) -> Result<()> {
        if !self.is_recording.load(Ordering::SeqCst) {
            return Err(AppError::InvalidState("Not recording".into()));
        }
        self.command_tx
            .send(AudioCommand::Resume)
            .map_err(|_| AppError::Audio("Audio thread not responding".into()))?;
        self.is_paused.store(false, Ordering::SeqCst);
        info!("Recording resumed");
        Ok(())
    }

    pub fn get_audio_level(&self) -> f32 {
        *self.audio_level.lock()
    }

    pub fn sample_rate(&self) -> u32 {
        *self.sample_rate.lock()
    }

    pub fn is_recording(&self) -> bool {
        self.is_recording.load(Ordering::SeqCst)
    }
}

impl Drop for AudioCapture {
    fn drop(&mut self) {
        let _ = self.command_tx.send(AudioCommand::Shutdown);
    }
}

impl Default for AudioCapture {
    fn default() -> Self {
        Self::new()
    }
}

/// Audio thread that owns the cpal Stream
fn audio_thread(
    command_rx: Receiver<AudioCommand>,
    is_recording: Arc<AtomicBool>,
    is_paused: Arc<AtomicBool>,
    audio_level: Arc<Mutex<f32>>,
    sample_rate: Arc<Mutex<u32>>,
) {
    let mut current_stream: Option<cpal::Stream> = None;
    let buffer: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));

    // Generation counter to prevent stale callbacks from writing to buffer
    // Each new recording increments this counter
    let recording_generation: Arc<AtomicU64> = Arc::new(AtomicU64::new(0));

    loop {
        match command_rx.recv() {
            Ok(AudioCommand::Start { device_id, response }) => {
                // 1. Increment generation FIRST to invalidate any in-flight callbacks
                let new_generation = recording_generation.fetch_add(1, Ordering::SeqCst) + 1;
                info!("Starting recording generation {}", new_generation);

                // 2. Properly stop any existing stream: pause THEN drop
                if let Some(stream) = current_stream.take() {
                    is_recording.store(false, Ordering::SeqCst);
                    if let Err(e) = stream.pause() {
                        warn!("Failed to pause old stream: {}", e);
                    }
                    drop(stream);
                    std::thread::sleep(Duration::from_millis(50));
                }

                // 3. Clear buffer BEFORE creating new stream (critical!)
                buffer.lock().clear();
                debug!("Buffer cleared for generation {}", new_generation);

                // 4. Create and start stream with current generation
                let result = start_stream(
                    device_id.as_deref(),
                    Arc::clone(&buffer),
                    Arc::clone(&is_recording),
                    Arc::clone(&is_paused),
                    Arc::clone(&audio_level),
                    Arc::clone(&sample_rate),
                    Arc::clone(&recording_generation),
                    new_generation,
                );

                match result {
                    Ok(stream) => {
                        // 5. Store stream, THEN enable recording flag
                        current_stream = Some(stream);
                        is_paused.store(false, Ordering::SeqCst);
                        is_recording.store(true, Ordering::SeqCst);
                        info!("Recording generation {} started", new_generation);
                        let _ = response.send(Ok(()));
                    }
                    Err(e) => {
                        let _ = response.send(Err(e));
                    }
                }
            }
            Ok(AudioCommand::Stop { response }) => {
                let gen = recording_generation.load(Ordering::SeqCst);
                info!("Stopping recording generation {}", gen);

                // 1. Stop accepting new samples immediately
                is_recording.store(false, Ordering::SeqCst);

                // 2. Properly stop stream: pause THEN drop
                if let Some(stream) = current_stream.take() {
                    if let Err(e) = stream.pause() {
                        warn!("Failed to pause stream: {}", e);
                    }
                    drop(stream);
                }

                // 3. Small delay to let in-flight callbacks complete
                std::thread::sleep(Duration::from_millis(50));

                // 4. Take all samples from buffer
                let samples = std::mem::take(&mut *buffer.lock());
                info!("Recording generation {} stopped: {} samples ({:.2}s @ 16kHz)",
                      gen,
                      samples.len(),
                      samples.len() as f32 / 16000.0);
                let _ = response.send(Ok(samples));
            }
            Ok(AudioCommand::Pause) => {
                if let Some(ref stream) = current_stream {
                    let _ = stream.pause();
                }
            }
            Ok(AudioCommand::Resume) => {
                if let Some(ref stream) = current_stream {
                    let _ = stream.play();
                }
            }
            Ok(AudioCommand::Shutdown) | Err(_) => {
                info!("Audio thread shutting down");
                break;
            }
        }
    }
}

fn start_stream(
    device_id: Option<&str>,
    buffer: Arc<Mutex<Vec<f32>>>,
    is_recording: Arc<AtomicBool>,
    is_paused: Arc<AtomicBool>,
    audio_level: Arc<Mutex<f32>>,
    sample_rate: Arc<Mutex<u32>>,
    recording_generation: Arc<AtomicU64>,
    expected_generation: u64,
) -> Result<cpal::Stream> {
    let host = cpal::default_host();

    let device = if let Some(id) = device_id {
        host.input_devices()
            .map_err(|e| AppError::Audio(e.to_string()))?
            .find(|d| d.name().map(|n| n == id).unwrap_or(false))
            .ok_or_else(|| AppError::NotFound(format!("Device not found: {}", id)))?
    } else {
        host.default_input_device()
            .ok_or_else(|| AppError::Audio("No default input device".into()))?
    };

    info!("Using audio device: {:?}", device.name());

    let config = device
        .default_input_config()
        .map_err(|e| AppError::Audio(e.to_string()))?;

    *sample_rate.lock() = config.sample_rate().0;
    info!("Audio config: {}Hz, {} channels, {:?}",
          config.sample_rate().0,
          config.channels(),
          config.sample_format());

    let err_fn = |err| warn!("Audio stream error: {}", err);
    let config_clone: StreamConfig = config.clone().into();

    let stream = match config.sample_format() {
        SampleFormat::F32 => build_stream_f32(
            &device,
            &config_clone,
            buffer,
            is_recording,
            is_paused,
            audio_level,
            recording_generation,
            expected_generation,
            err_fn,
        )?,
        SampleFormat::I16 => build_stream_i16(
            &device,
            &config_clone,
            buffer,
            is_recording,
            is_paused,
            audio_level,
            recording_generation,
            expected_generation,
            err_fn,
        )?,
        _ => return Err(AppError::Audio("Unsupported sample format".into())),
    };

    stream
        .play()
        .map_err(|e| AppError::Audio(e.to_string()))?;

    info!("Recording started");
    Ok(stream)
}

fn build_stream_f32<E>(
    device: &cpal::Device,
    config: &StreamConfig,
    buffer: Arc<Mutex<Vec<f32>>>,
    is_recording: Arc<AtomicBool>,
    is_paused: Arc<AtomicBool>,
    audio_level: Arc<Mutex<f32>>,
    recording_generation: Arc<AtomicU64>,
    expected_generation: u64,
    err_fn: E,
) -> Result<cpal::Stream>
where
    E: FnMut(cpal::StreamError) + Send + 'static,
{
    device
        .build_input_stream(
            config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                // Check generation FIRST - if it doesn't match, this callback is stale
                if recording_generation.load(Ordering::SeqCst) != expected_generation {
                    debug!("Stale callback rejected (gen {} != {})", expected_generation, recording_generation.load(Ordering::SeqCst));
                    return;
                }

                if !is_recording.load(Ordering::SeqCst) || is_paused.load(Ordering::SeqCst) {
                    return;
                }

                // Calculate audio level (RMS) with gain boost for visualization
                let sum: f32 = data.iter().map(|s| s * s).sum();
                let rms = (sum / data.len() as f32).sqrt();
                // Apply gain (10x) and use sqrt for more visual range
                let boosted = (rms * 10.0).sqrt().min(1.0);
                *audio_level.lock() = boosted;

                buffer.lock().extend_from_slice(data);
            },
            err_fn,
            None,
        )
        .map_err(|e| AppError::Audio(e.to_string()))
}

fn build_stream_i16<E>(
    device: &cpal::Device,
    config: &StreamConfig,
    buffer: Arc<Mutex<Vec<f32>>>,
    is_recording: Arc<AtomicBool>,
    is_paused: Arc<AtomicBool>,
    audio_level: Arc<Mutex<f32>>,
    recording_generation: Arc<AtomicU64>,
    expected_generation: u64,
    err_fn: E,
) -> Result<cpal::Stream>
where
    E: FnMut(cpal::StreamError) + Send + 'static,
{
    device
        .build_input_stream(
            config,
            move |data: &[i16], _: &cpal::InputCallbackInfo| {
                // Check generation FIRST - if it doesn't match, this callback is stale
                if recording_generation.load(Ordering::SeqCst) != expected_generation {
                    debug!("Stale callback rejected (gen {} != {})", expected_generation, recording_generation.load(Ordering::SeqCst));
                    return;
                }

                if !is_recording.load(Ordering::SeqCst) || is_paused.load(Ordering::SeqCst) {
                    return;
                }

                let samples: Vec<f32> = data.iter().map(|&s| s as f32 / 32768.0).collect();

                // Calculate audio level (RMS) with gain boost for visualization
                let sum: f32 = samples.iter().map(|s| s * s).sum();
                let rms = (sum / samples.len() as f32).sqrt();
                // Apply gain (10x) and use sqrt for more visual range
                let boosted = (rms * 10.0).sqrt().min(1.0);
                *audio_level.lock() = boosted;

                buffer.lock().extend(samples);
            },
            err_fn,
            None,
        )
        .map_err(|e| AppError::Audio(e.to_string()))
}
