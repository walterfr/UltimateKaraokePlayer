use serde::{Serialize, Deserialize};
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackerInstrument {
    pub name: String,
    pub sample_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackerMetadata {
    pub title: String,
    pub format_name: String, // "MOD", "S3M", "XM", "ST3"
    pub channel_count: u32,
    pub pattern_count: u32,
    pub instrument_count: u32,
    pub estimated_duration: f64,
    pub instruments: Vec<TrackerInstrument>,
    pub file_size: u64,
    pub tracker_message: String,
}

pub struct TrackerParser;

impl TrackerParser {
    pub fn parse_file(path: &str) -> Result<TrackerMetadata, String> {
        let data = fs::read(path).map_err(|e| format!("Failed to read file: {}", e))?;
        let file_size = data.len() as u64;

        if data.len() < 4 {
            return Err("File too small to be a tracker module".to_string());
        }

        // Detect format by signature
        let sig = &data[0..4];
        let ext = path.split('.').last().unwrap_or("").to_lowercase();

        match sig {
            b"SCRM" => Self::parse_s3m(&data, &ext, file_size),
            [b'E', b'x', b't', b'e'] => Self::parse_xm(&data, &ext, file_size),
            _ => {
                // MOD format typically starts with a name, followed by format marker
                if ext == "mod" || data.len() > 1080 {
                    Self::parse_mod(&data, &ext, file_size)
                } else {
                    Err(format!("Unknown tracker format (sig: {:02X?})", &data[..4.min(data.len())]))
                }
            }
        }
    }

    fn parse_mod(data: &[u8], _ext: &str, file_size: u64) -> Result<TrackerMetadata, String> {
        let title = Self::read_string(data, 0, 20);
        
        // Check format marker at byte 1080 (for standard MOD)
        let channel_count = if data.len() > 1084 {
            let marker = &data[1080..1084];
            match marker {
                b"M.K." | b"M!K!" => 4,
                b"FLT4" => 4,
                b"FLT8" => 8,
                _ => 4, // default assumption
            }
        } else {
            4
        };

        let pattern_count = if data.len() > 1084 { data[950] as u32 } else { 0 };
        
        let mut instruments = Vec::new();
        // 31 instruments starting at byte 20, each 30 bytes
        for i in 0..31 {
            let start = 20 + i * 30;
            if start + 22 <= data.len() {
                let inst_name = Self::read_string(data, start, 22);
                let sample_len_bytes = [data[start + 22], data[start + 23]];
                let sample_len = u16::from_be_bytes(sample_len_bytes) as u32 * 2; // in words
                if !inst_name.is_empty() || sample_len > 0 {
                    instruments.push(TrackerInstrument {
                        name: inst_name,
                        sample_count: if sample_len > 0 { 1 } else { 0 },
                    });
                }
            }
        }

        Ok(TrackerMetadata {
            title,
            format_name: "MOD".to_string(),
            channel_count,
            pattern_count,
            instrument_count: instruments.len() as u32,
            estimated_duration: 0.0,
            instruments,
            file_size,
            tracker_message: String::new(),
        })
    }

    fn parse_s3m(data: &[u8], _ext: &str, file_size: u64) -> Result<TrackerMetadata, String> {
        let title = Self::read_string(data, 2, 28);
        
        let channel_count = if data.len() > 68 {
            // Channel settings at offset 68 (byte per channel: 0xFF = disabled, 0x80+n = enabled)
            let count = data[64..96].iter()
                .filter(|&&b| b != 0xFF && b > 0)
                .count() as u32;
            count
        } else {
            4
        };

        let _ = data; // more parsing would go here

        Ok(TrackerMetadata {
            title,
            format_name: "S3M".to_string(),
            channel_count,
            pattern_count: 0,
            instrument_count: 0,
            estimated_duration: 0.0,
            instruments: vec![],
            file_size,
            tracker_message: String::new(),
        })
    }

    fn parse_xm(data: &[u8], _ext: &str, file_size: u64) -> Result<TrackerMetadata, String> {
        let title = Self::read_string(data, 17, 20);

        let channel_count = if data.len() > 64 { 
            u16::from_le_bytes([data[64], data[65]]) as u32 
        } else { 4 };
        
        let pattern_count = if data.len() > 68 {
            u16::from_le_bytes([data[66], data[67]]) as u32
        } else { 0 };

        let instrument_count = if data.len() > 70 {
            u16::from_le_bytes([data[68], data[69]]) as u32
        } else { 0 };

        let sample_count = if data.len() > 72 {
            u16::from_le_bytes([data[70], data[71]]) as u32
        } else { 0 };

        let instruments = vec![TrackerInstrument {
            name: format!("{} samples", sample_count),
            sample_count,
        }];

        Ok(TrackerMetadata {
            title,
            format_name: "XM".to_string(),
            channel_count,
            pattern_count,
            instrument_count,
            estimated_duration: 0.0,
            instruments,
            file_size,
            tracker_message: String::new(),
        })
    }

    fn read_string(data: &[u8], offset: usize, max_len: usize) -> String {
        let end = (offset + max_len).min(data.len());
        let slice = &data[offset..end];
        let null_pos = slice.iter().position(|&b| b == 0).unwrap_or(slice.len());
        String::from_utf8_lossy(&slice[..null_pos]).trim().to_string()
    }
}
