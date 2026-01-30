use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Segment {
    pub id: String,
    pub start_ms: i64,
    pub end_ms: i64,
    pub text: String,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Transcription {
    pub id: String,
    pub created_at: String,
    pub updated_at: String,
    pub source_type: String, // "dictation" or "file"
    pub source_name: Option<String>,
    pub duration_ms: i64,
    pub language: String,
    pub segments: Vec<Segment>,
    pub raw_text: String,
    pub edited_text: Option<String>,
    pub is_edited: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub theme: String,
    pub language: String,
    pub input_device_id: Option<String>,
    pub shortcuts: ShortcutSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShortcutSettings {
    pub toggle_recording: String,
    pub pause: String,
    pub copy: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: "system".to_string(),
            language: "fr".to_string(),
            input_device_id: None,
            shortcuts: ShortcutSettings {
                toggle_recording: "CommandOrControl+Shift+R".to_string(),
                pause: "CommandOrControl+Shift+P".to_string(),
                copy: "CommandOrControl+Shift+C".to_string(),
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioDevice {
    pub id: String,
    pub name: String,
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptionProgress {
    pub current_ms: i64,
    pub total_ms: i64,
    pub speed_factor: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamingSegment {
    pub text: String,
    pub is_final: bool,
    pub confidence: Option<f64>,
}
