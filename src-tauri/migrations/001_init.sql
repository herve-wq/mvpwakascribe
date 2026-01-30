-- WakaScribe Database Schema

-- Table des transcriptions
CREATE TABLE IF NOT EXISTS transcriptions (
    id TEXT PRIMARY KEY,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    source_type TEXT NOT NULL CHECK (source_type IN ('dictation', 'file')),
    source_name TEXT,
    duration_ms INTEGER,
    language TEXT DEFAULT 'fr',
    raw_text TEXT,
    edited_text TEXT,
    is_edited INTEGER DEFAULT 0
);

-- Table des segments avec timestamps et confiance
CREATE TABLE IF NOT EXISTS segments (
    id TEXT PRIMARY KEY,
    transcription_id TEXT NOT NULL,
    start_ms INTEGER NOT NULL,
    end_ms INTEGER NOT NULL,
    text TEXT NOT NULL,
    confidence REAL,
    FOREIGN KEY (transcription_id) REFERENCES transcriptions(id) ON DELETE CASCADE
);

-- Table des préférences utilisateur
CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value TEXT
);

-- Index pour la recherche
CREATE INDEX IF NOT EXISTS idx_transcriptions_created ON transcriptions(created_at);
CREATE INDEX IF NOT EXISTS idx_segments_transcription ON segments(transcription_id);
