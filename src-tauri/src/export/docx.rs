use crate::error::{AppError, Result};
use crate::storage::Transcription;
use docx_rs::*;
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

pub fn export_to_docx(transcription: &Transcription, path: &Path) -> Result<()> {
    let text = transcription
        .edited_text
        .as_ref()
        .unwrap_or(&transcription.raw_text);

    let mut docx = Docx::new();

    // Title
    docx = docx.add_paragraph(
        Paragraph::new()
            .add_run(Run::new().add_text("Transcription WakaScribe").bold()),
    );

    // Metadata
    docx = docx.add_paragraph(Paragraph::new());
    docx = docx.add_paragraph(
        Paragraph::new().add_run(Run::new().add_text(format!("Date: {}", transcription.created_at))),
    );

    if let Some(ref name) = transcription.source_name {
        docx = docx.add_paragraph(
            Paragraph::new().add_run(Run::new().add_text(format!("Source: {}", name))),
        );
    }

    docx = docx.add_paragraph(
        Paragraph::new().add_run(
            Run::new().add_text(format!("Duree: {}", format_duration(transcription.duration_ms))),
        ),
    );

    // Separator
    docx = docx.add_paragraph(Paragraph::new());

    // Main content
    for paragraph in text.split('\n') {
        if !paragraph.is_empty() {
            docx = docx.add_paragraph(Paragraph::new().add_run(Run::new().add_text(paragraph)));
        }
    }

    // Segments
    if !transcription.segments.is_empty() {
        docx = docx.add_paragraph(Paragraph::new());
        docx = docx.add_paragraph(
            Paragraph::new().add_run(Run::new().add_text("Segments detailles:").bold()),
        );
        docx = docx.add_paragraph(Paragraph::new());

        for segment in &transcription.segments {
            let segment_text = format!(
                "[{}] {} (confiance: {:.0}%)",
                format_timestamp(segment.start_ms),
                segment.text,
                segment.confidence * 100.0
            );
            docx =
                docx.add_paragraph(Paragraph::new().add_run(Run::new().add_text(segment_text)));
        }
    }

    // Write to file
    let file = File::create(path)?;
    let writer = BufWriter::new(file);
    docx.build()
        .pack(writer)
        .map_err(|e| AppError::Export(format!("Failed to create docx: {:?}", e)))?;

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
