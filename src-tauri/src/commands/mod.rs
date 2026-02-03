pub mod audio;
pub mod export;
pub mod history;
pub mod settings;
pub mod transcription;

// Module de test - commenter cette ligne pour désactiver
pub mod test_transcription;

pub use audio::*;
pub use export::*;
pub use history::*;
pub use settings::*;
pub use transcription::*;

// Export test - commenter cette ligne pour désactiver
pub use test_transcription::*;
