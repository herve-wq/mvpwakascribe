use crate::error::Result;
use crate::storage::Transcription;
use std::fs::File;
use std::io::Write;
use std::path::Path;

pub fn export_to_txt(transcription: &Transcription, path: &Path) -> Result<()> {
    let mut file = File::create(path)?;

    // Header
    writeln!(file, "Transcription WakaScribe")?;
    writeln!(file, "========================")?;
    writeln!(file)?;
    writeln!(file, "Date: {}", transcription.created_at)?;
    if let Some(ref name) = transcription.source_name {
        writeln!(file, "Source: {}", name)?;
    }
    writeln!(
        file,
        "Duree: {}",
        format_duration(transcription.duration_ms)
    )?;
    writeln!(file)?;
    writeln!(file, "---")?;
    writeln!(file)?;

    // Content
    let text = transcription
        .edited_text
        .as_ref()
        .unwrap_or(&transcription.raw_text);
    writeln!(file, "{}", text)?;

    // Segments with timestamps
    if !transcription.segments.is_empty() {
        writeln!(file)?;
        writeln!(file, "---")?;
        writeln!(file)?;
        writeln!(file, "Segments detailles:")?;
        writeln!(file)?;

        for segment in &transcription.segments {
            writeln!(
                file,
                "[{}] {} (confiance: {:.0}%)",
                format_timestamp(segment.start_ms),
                segment.text,
                segment.confidence * 100.0
            )?;
        }
    }

    Ok(())
}

fn format_duration(ms: i64) -> String {
    let total_seconds = ms / 1000;
    let minutes = total_seconds / 60;
    let seconds = total_seconds % 60;
    format!("{}:{:02}", minutes, seconds)
}

fn format_timestamp(ms: i64) -> String {
    let total_seconds = ms / 1000;
    let minutes = total_seconds / 60;
    let seconds = total_seconds % 60;
    format!("{:02}:{:02}", minutes, seconds)
}
