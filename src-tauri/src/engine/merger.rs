//! Transcription merger for chunked audio
//!
//! Merges transcription results from overlapping audio chunks,
//! handling deduplication in overlap regions.

use tracing::{debug, info};

/// A transcribed chunk with timing information
#[derive(Debug, Clone)]
pub struct ChunkTranscription {
    /// The transcribed text
    pub text: String,
    /// Start time in the original audio (ms)
    pub start_ms: i64,
    /// End time in the original audio (ms)
    pub end_ms: i64,
    /// Chunk index
    pub index: usize,
}

/// Merge transcriptions from overlapping chunks
///
/// Strategy:
/// - For non-overlapping regions: use text as-is
/// - For overlapping regions: prefer the chunk where that region is more "central"
///   (i.e., not at the edge where transcription quality may be lower)
///
/// # Arguments
/// * `chunks` - Vector of chunk transcriptions in order
/// * `overlap_ms` - Overlap duration in milliseconds
///
/// # Returns
/// Merged transcription text
pub fn merge_transcriptions(chunks: &[ChunkTranscription], overlap_ms: i64) -> String {
    if chunks.is_empty() {
        return String::new();
    }

    if chunks.len() == 1 {
        return chunks[0].text.clone();
    }

    info!(
        "Merging {} chunk transcriptions with {}ms overlap",
        chunks.len(),
        overlap_ms
    );

    let mut merged_parts: Vec<String> = Vec::new();

    for (i, chunk) in chunks.iter().enumerate() {
        let text = chunk.text.trim();

        if text.is_empty() {
            continue;
        }

        if i == 0 {
            // First chunk: take all text, but may need to trim end for overlap
            let trimmed = trim_overlap_end(text, overlap_ms, chunk.end_ms - chunk.start_ms);
            debug!("Chunk {}: using '{}' (trimmed end)", i, trimmed);
            merged_parts.push(trimmed);
        } else if i == chunks.len() - 1 {
            // Last chunk: skip beginning overlap, take rest
            let trimmed = trim_overlap_start(text, overlap_ms, chunk.end_ms - chunk.start_ms);
            debug!("Chunk {}: using '{}' (trimmed start)", i, trimmed);
            merged_parts.push(trimmed);
        } else {
            // Middle chunks: trim both ends
            let trimmed_start =
                trim_overlap_start(text, overlap_ms / 2, chunk.end_ms - chunk.start_ms);
            let trimmed = trim_overlap_end(
                &trimmed_start,
                overlap_ms / 2,
                chunk.end_ms - chunk.start_ms,
            );
            debug!("Chunk {}: using '{}' (trimmed both)", i, trimmed);
            merged_parts.push(trimmed);
        }
    }

    // Join with spaces, cleaning up any double spaces
    let merged = merged_parts.join(" ");
    let cleaned = merge_cleanup(&merged);

    info!("Merged result: {} chars", cleaned.len());
    cleaned
}

/// Trim the end of text to account for overlap
/// We estimate that overlap_ms corresponds to roughly (overlap_ms / chunk_ms) of the text
fn trim_overlap_end(text: &str, overlap_ms: i64, chunk_duration_ms: i64) -> String {
    if overlap_ms <= 0 || chunk_duration_ms <= 0 {
        return text.to_string();
    }

    let words: Vec<&str> = text.split_whitespace().collect();
    if words.is_empty() {
        return String::new();
    }

    // Estimate how many words to trim based on overlap ratio
    let overlap_ratio = overlap_ms as f32 / chunk_duration_ms as f32;
    let words_to_trim = ((words.len() as f32 * overlap_ratio) / 2.0).ceil() as usize;
    let words_to_keep = words.len().saturating_sub(words_to_trim);

    if words_to_keep == 0 {
        return String::new();
    }

    words[..words_to_keep].join(" ")
}

/// Trim the start of text to account for overlap
fn trim_overlap_start(text: &str, overlap_ms: i64, chunk_duration_ms: i64) -> String {
    if overlap_ms <= 0 || chunk_duration_ms <= 0 {
        return text.to_string();
    }

    let words: Vec<&str> = text.split_whitespace().collect();
    if words.is_empty() {
        return String::new();
    }

    // Estimate how many words to trim based on overlap ratio
    let overlap_ratio = overlap_ms as f32 / chunk_duration_ms as f32;
    let words_to_trim = ((words.len() as f32 * overlap_ratio) / 2.0).ceil() as usize;

    if words_to_trim >= words.len() {
        return String::new();
    }

    words[words_to_trim..].join(" ")
}

/// Clean up merged text
fn merge_cleanup(text: &str) -> String {
    // Remove double spaces
    let mut result = text.to_string();
    while result.contains("  ") {
        result = result.replace("  ", " ");
    }

    // Remove space before punctuation
    result = result.replace(" .", ".");
    result = result.replace(" ,", ",");
    result = result.replace(" !", "!");
    result = result.replace(" ?", "?");
    result = result.replace(" :", ":");
    result = result.replace(" ;", ";");

    result.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_chunk() {
        let chunks = vec![ChunkTranscription {
            text: "Hello world".to_string(),
            start_ms: 0,
            end_ms: 5000,
            index: 0,
        }];
        let result = merge_transcriptions(&chunks, 2000);
        assert_eq!(result, "Hello world");
    }

    #[test]
    fn test_two_chunks_merge() {
        let chunks = vec![
            ChunkTranscription {
                text: "This is the first part of the sentence".to_string(),
                start_ms: 0,
                end_ms: 10000,
                index: 0,
            },
            ChunkTranscription {
                text: "part of the sentence and this continues".to_string(),
                start_ms: 8000,
                end_ms: 18000,
                index: 1,
            },
        ];
        let result = merge_transcriptions(&chunks, 2000);
        // Should trim overlap from both
        assert!(result.contains("first"));
        assert!(result.contains("continues"));
    }

    #[test]
    fn test_cleanup() {
        assert_eq!(merge_cleanup("hello  world"), "hello world");
        assert_eq!(merge_cleanup("hello ."), "hello.");
        assert_eq!(merge_cleanup("what ?"), "what?");
    }
}
