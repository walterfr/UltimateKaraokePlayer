use serde::{Serialize, Deserialize};
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyMetadata {
    pub title: String,
    pub artist: String,
    pub format: String, // "MK1", "KARA", "UNKNOWN"
    pub file_size: u64,
    pub detected_subformat: String, // "midi_based", "cdg_based", "raw_audio", "unknown"
    pub estimated_duration: f64,
    pub header_hex: String, // first 32 bytes as hex for debugging
    pub notes: String,
}

pub struct LegacyParser;

impl LegacyParser {
    pub fn parse_file(path: &str) -> Result<LegacyMetadata, String> {
        let data = fs::read(path).map_err(|e| format!("Failed to read file: {}", e))?;
        let file_size = data.len() as u64;
        let ext = path.split('.').last().unwrap_or("").to_lowercase();
        let header_hex = data.iter().take(32).map(|b| format!("{:02X}", b)).collect::<Vec<_>>().join(" ");

        match ext.as_str() {
            "mk1" => Self::parse_mk1(&data, file_size, &header_hex, path),
            "kara" => Self::parse_kara(&data, file_size, &header_hex, path),
            _ => {
                // Try to auto-detect
                if data.len() > 4 {
                    let sig = &data[0..4];
                    if sig == b"MK1 " || sig == b"MK1\x00" || sig.starts_with(b"MK1") {
                        Self::parse_mk1(&data, file_size, &header_hex, path)
                    } else {
                        Self::parse_kara(&data, file_size, &header_hex, path)
                    }
                } else {
                    Err("Unknown legacy format: file too small".to_string())
                }
            }
        }
    }

    fn parse_mk1(data: &[u8], file_size: u64, header_hex: &str, _path: &str) -> Result<LegacyMetadata, String> {
        let mut title = String::from("Unknown MK1 Song");
        let mut artist = String::new();

        // MK1 header structure (varies by vendor, but common pattern):
        // Bytes 0-3: Magic "MK1 " or similar
        // Bytes 4-35: Song title (32 bytes)
        // Bytes 36-67: Artist (32 bytes)
        // Then audio data chunks follow

        if data.len() > 68 {
            // Try to extract title (often starts at offset 4 or after magic)
            let mut offset = 4;
            // Skip any padding
            while offset < data.len() && data[offset] == 0x00 { offset += 1; }
            
            if offset + 32 <= data.len() {
                let raw_title = Self::read_ascii(data, offset, 32);
                if !raw_title.is_empty() {
                    title = raw_title;
                }
            }

            // Artist often follows title
            let artist_offset = offset + 32;
            if artist_offset + 32 <= data.len() {
                let raw_artist = Self::read_ascii(data, artist_offset, 32);
                if !raw_artist.is_empty() {
                    artist = raw_artist;
                }
            }
        }

        let notes;
        if file_size > 1024 * 100 {
            notes = format!("MK1 file with embedded audio ({}), needs conversion for playback", 
                if file_size > 1024 * 1024 { format!("{:.1} MB", file_size as f64 / (1024.0 * 1024.0)) } 
                else { format!("{} KB", file_size / 1024) });
        } else {
            notes = "MK1 metadata header parsed. File may contain MIDI-like data.".to_string();
        }

        Ok(LegacyMetadata {
            title,
            artist,
            format: "MK1".to_string(),
            file_size,
            detected_subformat: "raw_audio".to_string(),
            estimated_duration: 0.0,
            header_hex: header_hex.to_string(),
            notes,
        })
    }

    fn parse_kara(data: &[u8], file_size: u64, header_hex: &str, _path: &str) -> Result<LegacyMetadata, String> {
        let mut title = String::from("Unknown KARA File");
        let mut artist = String::new();
        let mut subformat = "unknown";
        let mut notes = String::new();

        // KARA files are often one of:
        // 1. MIDI-based (starts with "MThd")
        // 2. CDG-based (starts with CDG magic or has CDG packet structure)
        // 3. Plain text with timing

        if data.len() > 4 {
            let sig = &data[0..4];
            if sig == b"MThd" {
                subformat = "midi_based";
                // Try to extract title from MIDI header
                if data.len() > 22 {
                    let raw = Self::read_ascii(data, 18, 20);
                    if !raw.is_empty() {
                        title = format!("[MIDI] {}", raw);
                    } else {
                        title = "MIDI-based KARA file".to_string();
                    }
                }
                notes = "KARA file detected as MIDI-based (MThd signature). Use MIDI engine.".to_string();
            } else if data.len() > 16 && data[0] == 0x00 && data[1] == 0x01 {
                // Likely CDG-based (first CDG packet)
                subformat = "cdg_based";
                title = "CDG-based KARA file".to_string();
                notes = "KARA file detected as CDG-based. Use CDG engine.".to_string();
            } else {
                subformat = "text_based";
                // Try to read as UTF-8 text
                let text = String::from_utf8_lossy(&data[..data.len().min(256)]);
                for line in text.lines() {
                    if line.starts_with("#TITLE") || line.starts_with("title:") {
                        title = line.split(':').nth(1).unwrap_or("").trim().to_string();
                    }
                    if line.starts_with("#ARTIST") || line.starts_with("artist:") {
                        artist = line.split(':').nth(1).unwrap_or("").trim().to_string();
                    }
                }
                if title == "Unknown KARA File" {
                    notes = "Text-based KARA format with embedded timing data.".to_string();
                }
            }
        }

        Ok(LegacyMetadata {
            title,
            artist,
            format: "KARA".to_string(),
            file_size,
            detected_subformat: subformat.to_string(),
            estimated_duration: 0.0,
            header_hex: header_hex.to_string(),
            notes,
        })
    }

    fn read_ascii(data: &[u8], offset: usize, max_len: usize) -> String {
        let end = (offset + max_len).min(data.len());
        let slice = &data[offset..end];
        let null_pos = slice.iter().position(|&b| b == 0x00 || !b.is_ascii_graphic() && b != b' ')
            .unwrap_or(slice.len());
        String::from_utf8_lossy(&slice[..null_pos]).trim().to_string()
    }
}
