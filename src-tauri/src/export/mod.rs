pub mod docx;
pub mod txt;

pub use self::docx::export_to_docx;
pub use txt::export_to_txt;

/// Format duration as "M:SS" (e.g. "2:05")
pub fn format_duration(ms: i64) -> String {
    let total_seconds = ms / 1000;
    let minutes = total_seconds / 60;
    let seconds = total_seconds % 60;
    format!("{}:{:02}", minutes, seconds)
}

/// Format timestamp as "MM:SS" (e.g. "02:05")
pub fn format_timestamp(ms: i64) -> String {
    let total_seconds = ms / 1000;
    let minutes = total_seconds / 60;
    let seconds = total_seconds % 60;
    format!("{:02}:{:02}", minutes, seconds)
}
