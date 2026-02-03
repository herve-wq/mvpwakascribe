pub mod capture;
pub mod chunker;
pub mod processor;
pub mod vad;

pub use capture::AudioCapture;
pub use chunker::{split_audio_smart, SmartChunkConfig};
pub use processor::{duration_ms, load_audio_file, normalize_audio, resample_to_16k, write_wav};
