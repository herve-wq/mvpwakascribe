use crate::error::Result;
use crate::storage::models::{Segment, Settings, Transcription};
use rusqlite::{params, Connection};

// Transcription queries

pub fn insert_transcription(conn: &Connection, t: &Transcription) -> Result<()> {
    conn.execute(
        r#"
        INSERT INTO transcriptions (id, created_at, updated_at, source_type, source_name, duration_ms, language, raw_text, edited_text, is_edited)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
        "#,
        params![
            t.id,
            t.created_at,
            t.updated_at,
            t.source_type,
            t.source_name,
            t.duration_ms,
            t.language,
            t.raw_text,
            t.edited_text,
            t.is_edited as i32
        ],
    )?;

    // Insert segments
    for seg in &t.segments {
        conn.execute(
            r#"
            INSERT INTO segments (id, transcription_id, start_ms, end_ms, text, confidence)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            "#,
            params![seg.id, t.id, seg.start_ms, seg.end_ms, seg.text, seg.confidence],
        )?;
    }

    Ok(())
}

pub fn get_transcription(conn: &Connection, id: &str) -> Result<Option<Transcription>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT id, created_at, updated_at, source_type, source_name, duration_ms, language, raw_text, edited_text, is_edited
        FROM transcriptions
        WHERE id = ?1
        "#,
    )?;

    let transcription = stmt.query_row([id], |row| {
        Ok(Transcription {
            id: row.get(0)?,
            created_at: row.get(1)?,
            updated_at: row.get(2)?,
            source_type: row.get(3)?,
            source_name: row.get(4)?,
            duration_ms: row.get(5)?,
            language: row.get(6)?,
            raw_text: row.get(7)?,
            edited_text: row.get(8)?,
            is_edited: row.get::<_, i32>(9)? != 0,
            segments: vec![],
        })
    });

    match transcription {
        Ok(mut t) => {
            t.segments = get_segments(conn, &t.id)?;
            Ok(Some(t))
        }
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

pub fn list_transcriptions(conn: &Connection) -> Result<Vec<Transcription>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT id, created_at, updated_at, source_type, source_name, duration_ms, language, raw_text, edited_text, is_edited
        FROM transcriptions
        ORDER BY created_at DESC
        "#,
    )?;

    let rows = stmt.query_map([], |row| {
        Ok(Transcription {
            id: row.get(0)?,
            created_at: row.get(1)?,
            updated_at: row.get(2)?,
            source_type: row.get(3)?,
            source_name: row.get(4)?,
            duration_ms: row.get(5)?,
            language: row.get(6)?,
            raw_text: row.get(7)?,
            edited_text: row.get(8)?,
            is_edited: row.get::<_, i32>(9)? != 0,
            segments: vec![],
        })
    })?;

    let mut transcriptions = Vec::new();
    for row in rows {
        let mut t = row?;
        t.segments = get_segments(conn, &t.id)?;
        transcriptions.push(t);
    }

    Ok(transcriptions)
}

fn get_segments(conn: &Connection, transcription_id: &str) -> Result<Vec<Segment>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT id, start_ms, end_ms, text, confidence
        FROM segments
        WHERE transcription_id = ?1
        ORDER BY start_ms
        "#,
    )?;

    let rows = stmt.query_map([transcription_id], |row| {
        Ok(Segment {
            id: row.get(0)?,
            start_ms: row.get(1)?,
            end_ms: row.get(2)?,
            text: row.get(3)?,
            confidence: row.get(4)?,
        })
    })?;

    Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
}

pub fn update_transcription_text(conn: &Connection, id: &str, edited_text: &str) -> Result<()> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        r#"
        UPDATE transcriptions
        SET edited_text = ?1, is_edited = 1, updated_at = ?2
        WHERE id = ?3
        "#,
        params![edited_text, now, id],
    )?;
    Ok(())
}

pub fn delete_transcription(conn: &Connection, id: &str) -> Result<()> {
    conn.execute("DELETE FROM segments WHERE transcription_id = ?1", [id])?;
    conn.execute("DELETE FROM transcriptions WHERE id = ?1", [id])?;
    Ok(())
}

// Settings queries

pub fn get_settings(conn: &Connection) -> Result<Settings> {
    let mut stmt = conn.prepare("SELECT key, value FROM settings")?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;

    let mut settings = Settings::default();
    for row in rows {
        let (key, value) = row?;
        match key.as_str() {
            "theme" => settings.theme = value,
            "language" => settings.language = value,
            "input_device_id" => settings.input_device_id = Some(value),
            "shortcut_toggle_recording" => settings.shortcuts.toggle_recording = value,
            "shortcut_pause" => settings.shortcuts.pause = value,
            "shortcut_copy" => settings.shortcuts.copy = value,
            _ => {}
        }
    }

    Ok(settings)
}

pub fn update_settings(conn: &Connection, settings: &Settings) -> Result<()> {
    let pairs = vec![
        ("theme", settings.theme.clone()),
        ("language", settings.language.clone()),
        (
            "shortcut_toggle_recording",
            settings.shortcuts.toggle_recording.clone(),
        ),
        ("shortcut_pause", settings.shortcuts.pause.clone()),
        ("shortcut_copy", settings.shortcuts.copy.clone()),
    ];

    for (key, value) in pairs {
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
            params![key, value],
        )?;
    }

    if let Some(ref device_id) = settings.input_device_id {
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES ('input_device_id', ?1)",
            [device_id],
        )?;
    }

    Ok(())
}
