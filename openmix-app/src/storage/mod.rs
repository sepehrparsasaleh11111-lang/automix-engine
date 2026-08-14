pub mod db;

use std::path::Path;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid json: {0}")]
    InvalidJson(String),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Track {
    pub id: String,
    pub project_id: Option<String>,
    pub path: String,
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub duration_ms: i64,
    pub sample_rate: u32,
    pub channels: u16,
    pub format: String,
    pub file_hash: String,
    pub peaks: Vec<openmix_core::audio::Peak>,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AnalysisRow {
    pub track_id: String,
    pub file_hash: String,
    pub bpm: Option<f64>,
    pub bpm_confidence: Option<f32>,
    pub key: Option<String>,
    pub key_confidence: Option<f32>,
    pub energy: String,
    pub created_at: String,
}

pub struct Storage {
    conn: Mutex<Connection>,
}

pub(crate) fn now_stamp() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("{secs}")
}

impl Storage {
    pub fn open(path: &Path) -> Result<Self, StorageError> {
        let conn = Connection::open(path)?;
        db::migrate(&conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn open_in_memory() -> Result<Self, StorageError> {
        let conn = Connection::open_in_memory()?;
        db::migrate(&conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn create_project(&self, name: &str) -> Result<Project, StorageError> {
        let id = uuid::Uuid::new_v4().to_string();
        let stamp = now_stamp();
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO projects (id, name, created_at, updated_at) VALUES (?1, ?2, ?3, ?3)",
            params![id, name, stamp],
        )?;
        Ok(Project {
            id,
            name: name.to_string(),
            created_at: stamp.clone(),
            updated_at: stamp,
        })
    }

    pub fn list_projects(&self) -> Result<Vec<Project>, StorageError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT id, name, created_at, updated_at FROM projects ORDER BY created_at")?;
        let rows = stmt.query_map([], |row| {
            Ok(Project {
                id: row.get(0)?,
                name: row.get(1)?,
                created_at: row.get(2)?,
                updated_at: row.get(3)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn delete_project(&self, id: &str) -> Result<(), StorageError> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM projects WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn insert_track(&self, t: &Track) -> Result<(), StorageError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO tracks (id, project_id, path, title, artist, album, duration_ms, \
             sample_rate, channels, format, file_hash, peaks, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                t.id,
                t.project_id,
                t.path,
                t.title,
                t.artist,
                t.album,
                t.duration_ms,
                t.sample_rate,
                t.channels,
                t.format,
                t.file_hash,
                serde_json::to_string(&t.peaks).unwrap_or_default(),
                t.created_at,
            ],
        )?;
        Ok(())
    }

    pub fn list_tracks(&self, project_id: Option<&str>) -> Result<Vec<Track>, StorageError> {
        let conn = self.conn.lock().unwrap();
        let (sql, key) = match project_id {
            Some(pid) => (
                "SELECT id, project_id, path, title, artist, album, duration_ms, sample_rate, \
                 channels, format, file_hash, peaks, created_at FROM tracks \
                 WHERE project_id = ?1 ORDER BY created_at",
                Some(pid.to_string()),
            ),
            None => (
                "SELECT id, project_id, path, title, artist, album, duration_ms, sample_rate, \
                 channels, format, file_hash, peaks, created_at FROM tracks \
                 WHERE project_id IS NULL ORDER BY created_at",
                None,
            ),
        };
        let mut stmt = conn.prepare(sql)?;
        let rows = match key {
            Some(pid) => stmt.query_map(params![pid], row_to_track)?,
            None => stmt.query_map([], row_to_track)?,
        };
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn get_track(&self, track_id: &str) -> Result<Option<Track>, StorageError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, project_id, path, title, artist, album, duration_ms, sample_rate, \
             channels, format, file_hash, peaks, created_at FROM tracks WHERE id = ?1",
        )?;
        let mut rows = stmt.query(params![track_id])?;
        match rows.next()? {
            Some(row) => Ok(Some(row_to_track(row)?)),
            None => Ok(None),
        }
    }

    pub fn get_pref(&self, key: &str) -> Result<Option<String>, StorageError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT value FROM preferences WHERE key = ?1")?;
        let mut rows = stmt.query(params![key])?;
        match rows.next()? {
            Some(row) => Ok(Some(row.get(0)?)),
            None => Ok(None),
        }
    }

    pub fn set_pref(&self, key: &str, value: &str) -> Result<(), StorageError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO preferences (key, value) VALUES (?1, ?2) \
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn upsert_analysis(&self, row: &AnalysisRow) -> Result<(), StorageError> {
        check_json(&row.energy)?;
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO track_analysis (track_id, file_hash, bpm, bpm_confidence, key, \
             key_confidence, energy, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8) \
             ON CONFLICT(track_id) DO UPDATE SET \
             file_hash = excluded.file_hash, bpm = excluded.bpm, \
             bpm_confidence = excluded.bpm_confidence, key = excluded.key, \
             key_confidence = excluded.key_confidence, energy = excluded.energy, \
             created_at = excluded.created_at",
            params![
                row.track_id,
                row.file_hash,
                row.bpm,
                row.bpm_confidence,
                row.key,
                row.key_confidence,
                row.energy,
                row.created_at,
            ],
        )?;
        Ok(())
    }

    pub fn get_analysis(&self, track_id: &str) -> Result<Option<AnalysisRow>, StorageError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT track_id, file_hash, bpm, bpm_confidence, key, key_confidence, energy, \
             created_at FROM track_analysis WHERE track_id = ?1",
        )?;
        let mut rows = stmt.query(params![track_id])?;
        match rows.next()? {
            Some(row) => Ok(Some(row_to_analysis(row)?)),
            None => Ok(None),
        }
    }

    pub fn upsert_beat_grid(
        &self,
        track_id: &str,
        file_hash: &str,
        grid_json: &str,
    ) -> Result<(), StorageError> {
        check_json(grid_json)?;
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO beat_grids (track_id, file_hash, grid, created_at) \
             VALUES (?1, ?2, ?3, ?4) \
             ON CONFLICT(track_id) DO UPDATE SET \
             file_hash = excluded.file_hash, grid = excluded.grid, \
             created_at = excluded.created_at",
            params![track_id, file_hash, grid_json, now_stamp()],
        )?;
        Ok(())
    }

    pub fn get_beat_grid(&self, track_id: &str) -> Result<Option<String>, StorageError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT grid FROM beat_grids WHERE track_id = ?1")?;
        let mut rows = stmt.query(params![track_id])?;
        match rows.next()? {
            Some(row) => Ok(Some(row.get(0)?)),
            None => Ok(None),
        }
    }
}

fn row_to_track(row: &rusqlite::Row) -> rusqlite::Result<Track> {
    let peaks_json: String = row.get(11)?;
    let peaks = serde_json::from_str(&peaks_json).unwrap_or_default();
    Ok(Track {
        id: row.get(0)?,
        project_id: row.get(1)?,
        path: row.get(2)?,
        title: row.get(3)?,
        artist: row.get(4)?,
        album: row.get(5)?,
        duration_ms: row.get(6)?,
        sample_rate: row.get(7)?,
        channels: row.get(8)?,
        format: row.get(9)?,
        file_hash: row.get(10)?,
        peaks,
        created_at: row.get(12)?,
    })
}

fn row_to_analysis(row: &rusqlite::Row) -> rusqlite::Result<AnalysisRow> {
    Ok(AnalysisRow {
        track_id: row.get(0)?,
        file_hash: row.get(1)?,
        bpm: row.get(2)?,
        bpm_confidence: row.get(3)?,
        key: row.get(4)?,
        key_confidence: row.get(5)?,
        energy: row.get(6)?,
        created_at: row.get(7)?,
    })
}

fn check_json(s: &str) -> Result<(), StorageError> {
    if serde_json::from_str::<serde_json::Value>(s).is_err() {
        return Err(StorageError::InvalidJson(s.to_string()));
    }
    Ok(())
}
