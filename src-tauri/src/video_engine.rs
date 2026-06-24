use serde::{Serialize, Deserialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtitleCue {
    pub text: String,
    pub start_seconds: f64,
    pub end_seconds: f64,
    pub style: Option<String>, // for ASS override style
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoMetadata {
    pub title: String,
    pub duration: f64,
    pub width: u32,
    pub height: u32,
    pub has_video: bool,
    pub has_audio: bool,
    pub subtitles: Vec<SubtitleCue>,
    pub subtitle_format: String, // "lrc", "srt", "ass", "none"
}

pub struct VideoEngine;

impl VideoEngine {
    /// Parse an external subtitle file given the video path
    pub fn parse_subtitles(video_path: &str) -> Result<VideoMetadata, String> {
        let path = Path::new(video_path);
        let base_name = path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        let parent = path.parent().unwrap_or(Path::new("."));

        // Try to find subtitle files with same base name
        let extensions = ["lrc", "srt", "ass", "ssa"];
        let mut subtitles = Vec::new();
        let mut subtitle_format = "none".to_string();

        for ext in &extensions {
            let sub_path = parent.join(format!("{}.{}", base_name, ext));
            if sub_path.exists() {
                let sub_data = fs::read_to_string(&sub_path)
                    .map_err(|e| format!("Failed to read subtitle: {}", e))?;
                subtitles = match *ext {
                    "lrc" => Self::parse_lrc(&sub_data),
                    "srt" => Self::parse_srt(&sub_data),
                    "ass" | "ssa" => Self::parse_ass(&sub_data),
                    _ => continue,
                };
                subtitle_format = ext.to_string();
                break;
            }
        }

        let _metadata = fs::metadata(video_path).ok();

        Ok(VideoMetadata {
            title: base_name.replace('_', " ").replace('.', " "),
            duration: 0.0, // will be filled by frontend
            width: 1920,
            height: 1080,
            has_video: true,
            has_audio: true,
            subtitles,
            subtitle_format,
        })
    }

    fn parse_lrc(content: &str) -> Vec<SubtitleCue> {
        let mut cues = Vec::new();
        let re = regex::Regex::new(r"\[(\d+):(\d+\.\d+)\](.*)").unwrap();

        for line in content.lines() {
            if let Some(cap) = re.captures(line) {
                let minutes: f64 = cap[1].parse().unwrap_or(0.0);
                let seconds: f64 = cap[2].parse().unwrap_or(0.0);
                let text = cap[3].trim().to_string();
                if !text.is_empty() {
                    let start = minutes * 60.0 + seconds;
                    cues.push(SubtitleCue {
                        text,
                        start_seconds: start,
                        end_seconds: start + 3.0, // will be adjusted
                        style: None,
                    });
                }
            }
        }

        // Set end times based on next cue
        for i in 0..cues.len() {
            if i + 1 < cues.len() {
                cues[i].end_seconds = cues[i + 1].start_seconds;
            } else {
                cues[i].end_seconds = cues[i].start_seconds + 3.0;
            }
        }

        cues
    }

    fn parse_srt(content: &str) -> Vec<SubtitleCue> {
        let mut cues = Vec::new();
        let re = regex::Regex::new(
            r"(\d+)\n(\d+):(\d+):(\d+),(\d+) --> (\d+):(\d+):(\d+),(\d+)\n([\s\S]*?)(?:\n\n|\n?$)"
        ).unwrap();

        for cap in re.captures_iter(content) {
            let h1: f64 = cap[2].parse().unwrap_or(0.0);
            let m1: f64 = cap[3].parse().unwrap_or(0.0);
            let s1: f64 = cap[4].parse().unwrap_or(0.0);
            let ms1: f64 = cap[5].parse().unwrap_or(0.0);
            let start = h1 * 3600.0 + m1 * 60.0 + s1 + ms1 / 1000.0;

            let h2: f64 = cap[6].parse().unwrap_or(0.0);
            let m2: f64 = cap[7].parse().unwrap_or(0.0);
            let s2: f64 = cap[8].parse().unwrap_or(0.0);
            let ms2: f64 = cap[9].parse().unwrap_or(0.0);
            let end = h2 * 3600.0 + m2 * 60.0 + s2 + ms2 / 1000.0;

            let text = cap[10].trim().to_string();
            if !text.is_empty() {
                cues.push(SubtitleCue {
                    text,
                    start_seconds: start,
                    end_seconds: end,
                    style: None,
                });
            }
        }

        cues
    }

    fn parse_ass(content: &str) -> Vec<SubtitleCue> {
        let mut cues = Vec::new();
        let mut in_events = false;
        let re = regex::Regex::new(
            r"Dialogue:\s*\d+,(\d+):(\d+):(\d+)\.(\d+),(\d+):(\d+):(\d+)\.(\d+),(.*?),(.*?),(.*?),(.*?),(.*?),(.*?),(.*)"
        ).unwrap();

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("[Events]") {
                in_events = true;
                continue;
            }
            if in_events && trimmed.starts_with("Dialogue:") {
                if let Some(cap) = re.captures(trimmed) {
                    let h1: f64 = cap[1].parse().unwrap_or(0.0);
                    let m1: f64 = cap[2].parse().unwrap_or(0.0);
                    let s1: f64 = cap[3].parse().unwrap_or(0.0);
                    let ms1: f64 = cap[4].parse().unwrap_or(0.0);
                    let start = h1 * 3600.0 + m1 * 60.0 + s1 + ms1 / 100.0;

                    let h2: f64 = cap[5].parse().unwrap_or(0.0);
                    let m2: f64 = cap[6].parse().unwrap_or(0.0);
                    let s2: f64 = cap[7].parse().unwrap_or(0.0);
                    let ms2: f64 = cap[8].parse().unwrap_or(0.0);
                    let end = h2 * 3600.0 + m2 * 60.0 + s2 + ms2 / 100.0;

                    let style = cap[9].to_string();
                    // Remove ASS override codes like {\fn...}{\c...}
                    let text_raw = cap[15].to_string();
                    let clean = text_raw
                        .replace("\\N", "\n")
                        .replace("\\n", "\n")
                        .replace('\\', "");

                    // Strip {\\...} tags
                    let re_strip = regex::Regex::new(r"\{[^}]*\}").unwrap();
                    let text = re_strip.replace_all(&clean, "").to_string();

                    if !text.trim().is_empty() {
                        cues.push(SubtitleCue {
                            text: text.trim().to_string(),
                            start_seconds: start,
                            end_seconds: end,
                            style: Some(style),
                        });
                    }
                }
            }
        }

        cues
    }
}
