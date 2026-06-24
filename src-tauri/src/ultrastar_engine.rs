use serde::{Serialize, Deserialize};
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UltrastarNote {
    pub note_type: String,
    pub beat: f64,
    pub length: f64,
    pub pitch: i32,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UltrastarMetadata {
    pub title: String,
    pub artist: String,
    pub mp3: String,
    pub bpm: f64,
    pub gap: f64,
    pub video: String,
    pub cover: String,
    pub language: String,
    pub edition: String,
    pub genre: String,
    pub year: i32,
    pub creator: String,
    pub notes: Vec<UltrastarNote>,
    pub total_duration: f64,
}

pub struct UltrastarParser;

impl UltrastarParser {
    pub fn parse_file(path: &str) -> Result<UltrastarMetadata, String> {
        let bytes = fs::read(path).map_err(|e| format!("Failed to read file: {}", e))?;
        
        // Attempt to decode as Windows-1252 (which covers UTF-8 safely via decode)
        // or let encoding_rs guess, but WINDOWS_1252 decode handles Latin-1 perfectly.
        // Actually, we should try UTF-8 first. If it's valid UTF-8, use it. Otherwise fallback to 1252.
        let content = match std::str::from_utf8(&bytes) {
            Ok(utf8_str) => utf8_str.to_string(),
            Err(_) => {
                let (cow, _, _) = encoding_rs::WINDOWS_1252.decode(&bytes);
                cow.into_owned()
            }
        };

        let mut title = String::new();
        let mut artist = String::new();
        let mut mp3 = String::new();
        let mut bpm: f64 = 0.0;
        let mut gap: f64 = 0.0;
        let mut video = String::new();
        let mut cover = String::new();
        let mut language = String::new();
        let mut edition = String::new();
        let mut genre = String::new();
        let mut year: i32 = 0;
        let mut creator = String::new();
        let mut notes = Vec::new();
        for line in content.lines() {
            let trimmed = line.trim();

            if trimmed.is_empty() {
                continue;
            }

            if trimmed.starts_with('#') {
                if let Some(v) = trimmed.strip_prefix("#TITLE:") {
                    title = v.trim().to_string();
                } else if let Some(v) = trimmed.strip_prefix("#ARTIST:") {
                    artist = v.trim().to_string();
                } else if let Some(v) = trimmed.strip_prefix("#MP3:") {
                    mp3 = v.trim().to_string();
                } else if let Some(v) = trimmed.strip_prefix("#BPM:") {
                    bpm = v.trim().replace(',', ".").parse().unwrap_or(0.0);
                } else if let Some(v) = trimmed.strip_prefix("#GAP:") {
                    gap = v.trim().replace(',', ".").parse().unwrap_or(0.0);
                } else if let Some(v) = trimmed.strip_prefix("#VIDEO:") {
                    video = v.trim().to_string();
                } else if let Some(v) = trimmed.strip_prefix("#COVER:") {
                    cover = v.trim().to_string();
                } else if let Some(v) = trimmed.strip_prefix("#LANGUAGE:") {
                    language = v.trim().to_string();
                } else if let Some(v) = trimmed.strip_prefix("#EDITION:") {
                    edition = v.trim().to_string();
                } else if let Some(v) = trimmed.strip_prefix("#GENRE:") {
                    genre = v.trim().to_string();
                } else if let Some(v) = trimmed.strip_prefix("#YEAR:") {
                    year = v.trim().parse().unwrap_or(0);
                } else if let Some(v) = trimmed.strip_prefix("#CREATOR:") {
                    creator = v.trim().to_string();
                }
            } else {
                if trimmed == "E" {
                    break; // End of file indicator in Ultrastar
                }

                let note_type = trimmed.chars().next().unwrap_or(' ');
                if note_type == ':' || note_type == '*' || note_type == 'F' || note_type == 'R' || note_type == 'r' || note_type == 'G' {
                    let rest = &trimmed[1..].trim();
                    let parts: Vec<&str> = rest.splitn(4, ' ').collect();
                    if parts.len() >= 3 {
                        let beat: f64 = parts[0].parse().unwrap_or(0.0);
                        let length: f64 = parts[1].parse().unwrap_or(0.0);
                        let pitch: i32 = parts[2].parse().unwrap_or(0);
                        let text = if parts.len() >= 4 { parts[3].to_string() } else { String::new() };
                        notes.push(UltrastarNote {
                            note_type: note_type.to_string(),
                            beat,
                            length,
                            pitch,
                            text,
                        });
                    }
                } else if note_type == '-' {
                    // line break
                    notes.push(UltrastarNote {
                        note_type: "-".to_string(),
                        beat: 0.0,
                        length: 0.0,
                        pitch: 0,
                        text: String::new(),
                    });
                }
            }
        }

        let total_duration = if bpm > 0.0 {
            let beat_duration = 60.0 / (bpm * 4.0);
            let gap_seconds = gap / 1000.0;
            let last_beat = notes.iter().map(|n| n.beat + n.length).fold(0.0, f64::max);
            gap_seconds + last_beat * beat_duration + 2.0
        } else {
            0.0
        };

        Ok(UltrastarMetadata {
            title,
            artist,
            mp3,
            bpm,
            gap,
            video,
            cover,
            language,
            edition,
            genre,
            year,
            creator,
            notes,
            total_duration,
        })
    }
}
