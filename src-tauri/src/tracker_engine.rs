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

        let ext = path.split('.').last().unwrap_or("").to_lowercase();
        let format_name = ext.to_uppercase();

        let mut estimated_duration = 0.0;
        
        // Skip BASS for ST3 since it's a proprietary MIDI container, not a real tracker
        if ext != "st3" {
            if let Err(e) = crate::bass_engine::BassEngine::init() {
                println!("[BASS INIT ERROR] {}", e);
            }
            if let Ok(handle) = crate::bass_engine::BassEngine::load_music(path) {
                estimated_duration = crate::bass_engine::BassEngine::get_duration(handle);
                crate::bass_engine::BassEngine::stop(handle).unwrap_or_default();
            } else {
                return Err("BASS parse error".to_string());
            }
        }

        let title = std::path::Path::new(path).file_name()
            .unwrap_or_default().to_string_lossy().into_owned();

        Ok(TrackerMetadata {
            title,
            format_name,
            channel_count: 0,
            pattern_count: 0,
            instrument_count: 0,
            estimated_duration,
            instruments: vec![],
            file_size,
            tracker_message: "".to_string(),
        })
    }
}
