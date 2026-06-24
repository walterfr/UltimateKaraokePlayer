use serde::{Serialize, Deserialize};
use sqlx::{SqlitePool, sqlite::SqlitePoolOptions, Row};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SongEntry {
    pub id: i64,
    pub title: String,
    pub artist: String,
    pub file_path: String,
    pub file_type: String,
    pub duration: f64,
    pub size: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueEntry {
    pub id: i64,
    pub song_id: i64,
    pub position: i64,
    pub requested_by: String,
    pub status: String,
    pub song: Option<SongEntry>,
}

pub struct Library {
    pool: SqlitePool,
}

impl Library {
    pub async fn new(db_path: &str) -> Result<Self, String> {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect(db_path)
            .await
            .map_err(|e| format!("Failed to open database: {}", e))?;

        let lib = Self { pool };
        lib.init_schema().await?;
        Ok(lib)
    }

    async fn init_schema(&self) -> Result<(), String> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS songs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT NOT NULL DEFAULT '',
                artist TEXT NOT NULL DEFAULT '',
                file_path TEXT NOT NULL UNIQUE,
                file_type TEXT NOT NULL DEFAULT '',
                duration REAL NOT NULL DEFAULT 0,
                size INTEGER NOT NULL DEFAULT 0,
                last_modified TEXT,
                created_at TEXT DEFAULT (datetime('now'))
            )"
        ).execute(&self.pool).await.map_err(|e| e.to_string())?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS queue (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                song_id INTEGER NOT NULL,
                position INTEGER NOT NULL,
                requested_by TEXT NOT NULL DEFAULT '',
                status TEXT NOT NULL DEFAULT 'pending',
                created_at TEXT DEFAULT (datetime('now')),
                FOREIGN KEY (song_id) REFERENCES songs(id) ON DELETE CASCADE
            )"
        ).execute(&self.pool).await.map_err(|e| e.to_string())?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            )"
        ).execute(&self.pool).await.map_err(|e| e.to_string())?;

        Ok(())
    }

    #[allow(dead_code)]
    pub async fn add_song(&self, path: &str) -> Result<i64, String> {
        let p = Path::new(path);
        let file_name = p.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown");
        let ext = p.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
        let size = std::fs::metadata(path).map(|m| m.len() as i64).unwrap_or(0);
        let file_type = match ext.as_str() {
            "cdg" | "mp3" | "wav" | "ogg" | "flac" => "cdg",
            "mid" | "kar" | "smi" => "midi",
            "mp4" | "mkv" | "avi" | "mov" => "video",
            "mod" | "s3m" | "xm" | "st3" | "it" => "tracker",
            "mk1" | "kara" => "legacy",
            "txt" => "ultrastar",
            _ => "other",
        }.to_string();

        let result = sqlx::query(
            "INSERT OR IGNORE INTO songs (title, file_path, file_type, size) VALUES (?, ?, ?, ?)"
        )
        .bind(file_name.replace('_', " ").replace('.', " "))
        .bind(path)
        .bind(&file_type)
        .bind(size)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;

        Ok(result.last_insert_rowid())
    }

    pub async fn search_songs(&self, query: &str, sort_by: &str) -> Result<Vec<SongEntry>, String> {
        let pattern = format!("%{}%", query.to_lowercase());
        
        let order_clause = match sort_by {
            "title"  => "title ASC",
            "artist" => "artist ASC",
            "type"   => "file_type ASC, title ASC",
            "recent" => "id DESC",
            _        => "title ASC",
        };

        let query_str = format!(
            "SELECT id, title, artist, file_path, file_type, CAST(duration AS REAL), size FROM songs
             WHERE LOWER(title) LIKE ? OR LOWER(file_path) LIKE ?
             ORDER BY {} LIMIT 5000",
             order_clause
        );

        let rows = sqlx::query(&query_str)
        .bind(&pattern)
        .bind(&pattern)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;

        Ok(rows.iter().map(|r| SongEntry {
            id: r.try_get(0).unwrap_or(0),
            title: r.try_get(1).unwrap_or_else(|_| "Unknown".to_string()),
            artist: r.try_get(2).unwrap_or_else(|_| "".to_string()),
            file_path: r.try_get(3).unwrap_or_else(|_| "".to_string()),
            file_type: r.try_get(4).unwrap_or_else(|_| "".to_string()),
            duration: r.try_get(5).unwrap_or(0.0),
            size: r.try_get(6).unwrap_or(0),
        }).collect())
    }

    pub async fn enqueue(&self, song_id: i64, requested_by: &str) -> Result<i64, String> {
        let max_pos: Option<i64> = sqlx::query_scalar::<_, Option<i64>>(
            "SELECT COALESCE(MAX(position), 0) FROM queue WHERE status = 'pending'"
        )
            .fetch_one(&self.pool)
            .await
            .map_err(|e| e.to_string())?;

        let max_pos = max_pos.unwrap_or(0);

        let result = sqlx::query(
            "INSERT INTO queue (song_id, position, requested_by) VALUES (?, ?, ?)"
        )
        .bind(song_id)
        .bind(max_pos + 1)
        .bind(requested_by)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;

        Ok(result.last_insert_rowid())
    }

    pub async fn get_queue(&self) -> Result<Vec<QueueEntry>, String> {
        let rows = sqlx::query(
            "SELECT q.id, q.song_id, q.position, q.requested_by, q.status,
                    s.id as sid, s.title, s.artist, s.file_path, s.file_type, CAST(s.duration AS REAL), s.size
             FROM queue q
             LEFT JOIN songs s ON q.song_id = s.id
             WHERE q.status IN ('pending', 'playing')
             ORDER BY q.position ASC"
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;

        Ok(rows.iter().map(|r| QueueEntry {
            id: r.try_get(0).unwrap_or(0),
            song_id: r.try_get(1).unwrap_or(0),
            position: r.try_get(2).unwrap_or(0),
            requested_by: r.try_get(3).unwrap_or_else(|_| "".to_string()),
            status: r.try_get(4).unwrap_or_else(|_| "pending".to_string()),
            song: Some(SongEntry {
                id: r.try_get(5).unwrap_or(0),
                title: r.try_get(6).unwrap_or_else(|_| "Unknown".to_string()),
                artist: r.try_get(7).unwrap_or_else(|_| "".to_string()),
                file_path: r.try_get(8).unwrap_or_else(|_| "".to_string()),
                file_type: r.try_get(9).unwrap_or_else(|_| "".to_string()),
                duration: r.try_get(10).unwrap_or(0.0),
                size: r.try_get(11).unwrap_or(0),
            }),
        }).collect())
    }

    pub async fn remove_from_queue(&self, queue_id: i64) -> Result<(), String> {
        sqlx::query("DELETE FROM queue WHERE id = ?")
            .bind(queue_id)
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    #[allow(dead_code)]
    pub async fn clear_queue(&self) -> Result<(), String> {
        sqlx::query("DELETE FROM queue")
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    #[allow(dead_code)]
    pub async fn reorder_queue(&self, queue_id: i64, new_position: i64) -> Result<(), String> {
        // Simple swap-based reorder
        sqlx::query("UPDATE queue SET position = ? WHERE id = ?")
            .bind(new_position)
            .bind(queue_id)
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn scan_directory(&self, dir_path: &str, engine_filter: Option<String>) -> Result<usize, String> {
        println!("[SCAN] Iniciando scan de '{}' com engine={:?}", dir_path, engine_filter);

        let exts: &[&str] = match engine_filter.as_deref() {
            Some("cdg")       => &["mp3", "wav", "ogg", "flac"],
            Some("video")     => &["mp4", "mkv", "avi", "mov"],
            Some("midi")      => &["mid", "kar", "smi"],
            Some("tracker")   => &["mod", "s3m", "xm", "st3", "it"],
            Some("legacy")    => &["mk1", "kara"],
            Some("ultrastar") => &["txt"],
            _                 => &["mp3", "wav", "ogg", "flac", "mid", "kar", "mp4", "mkv", "avi", "mod", "s3m", "xm", "txt"],
        };

        // Coletar apenas caminhos — sem ler metadata do disco
        let mut dirs = vec![dir_path.to_string()];
        let mut files: Vec<(String, String, &str)> = Vec::new(); // (title, path, type)

        while let Some(current_dir) = dirs.pop() {
            match std::fs::read_dir(&current_dir) {
                Ok(entries) => {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.is_file() {
                            if let Some(ext_os) = path.extension() {
                                let ext_low = ext_os.to_string_lossy().to_lowercase();
                                if exts.contains(&ext_low.as_str()) {
                                    if let (Some(stem), Some(path_str)) = (
                                        path.file_stem().and_then(|s| s.to_str()),
                                        path.to_str()
                                    ) {
                                        let title = stem.replace('_', " ").replace('.', " ");
                                        let ftype = match engine_filter.as_deref() {
                                            Some(e) => e,
                                            None => match ext_low.as_str() {
                                                "mp3"|"wav"|"ogg"|"flac" => "cdg",
                                                "mid"|"kar"|"smi"        => "midi",
                                                "mp4"|"mkv"|"avi"|"mov"  => "video",
                                                "mod"|"s3m"|"xm"|"st3"|"it" => "tracker",
                                                "mk1"|"kara"             => "legacy",
                                                "txt"                    => "ultrastar",
                                                _                        => "other",
                                            },
                                        };
                                        files.push((title, path_str.to_string(), ftype));
                                    }
                                }
                            }
                        } else if path.is_dir() {
                            if let Some(p) = path.to_str() {
                                dirs.push(p.to_string());
                            }
                        }
                    }
                }
                Err(e) => println!("[SCAN] Erro ao ler '{}': {}", current_dir, e),
            }
        }

        println!("[SCAN] {} arquivos encontrados. Inserindo no banco...", files.len());

        if files.is_empty() {
            return Ok(0);
        }

        // Otimizações SQLite para inserção em massa
        sqlx::query("PRAGMA journal_mode=WAL").execute(&self.pool).await.ok();
        sqlx::query("PRAGMA synchronous=NORMAL").execute(&self.pool).await.ok();

        let mut count = 0usize;

        // Inserir em lotes de 1000
        for chunk in files.chunks(1000) {
            let mut tx = self.pool.begin().await.map_err(|e| e.to_string())?;
            for (title, path, ftype) in chunk {
                let result = sqlx::query(
                    "INSERT OR IGNORE INTO songs (title, file_path, file_type) VALUES (?, ?, ?)"
                )
                .bind(title)
                .bind(path)
                .bind(ftype)
                .execute(&mut *tx)
                .await
                .map_err(|e| e.to_string())?;

                if result.rows_affected() > 0 {
                    count += 1;
                }
            }
            tx.commit().await.map_err(|e| e.to_string())?;
        }

        println!("[SCAN] Concluído: {} novas músicas inseridas.", count);
        Ok(count)
    }

    pub async fn get_songs_count(&self) -> Result<i64, String> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM songs")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(count)
    }

    pub async fn clear_library(&self) -> Result<(), String> {
        sqlx::query("DELETE FROM songs")
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn save_setting(&self, key: &str, value: &str) -> Result<(), String> {
        sqlx::query("INSERT OR REPLACE INTO settings (key, value) VALUES (?, ?)")
            .bind(key)
            .bind(value)
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn get_all_settings(&self) -> Result<std::collections::HashMap<String, String>, String> {
        let rows = sqlx::query("SELECT key, value FROM settings")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())?;

        let mut map = std::collections::HashMap::new();
        for r in rows {
            if let (Ok(k), Ok(v)) = (r.try_get::<String, _>(0), r.try_get::<String, _>(1)) {
                map.insert(k, v);
            }
        }
        Ok(map)
    }
}
