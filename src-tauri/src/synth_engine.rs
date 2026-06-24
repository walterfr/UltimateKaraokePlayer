use serde::{Serialize, Deserialize};
use midly::{Smf, MetaMessage, MidiMessage};
use std::fs;
use encoding_rs::WINDOWS_1252;

/// Decodifica bytes de letras MIDI: tenta UTF-8 primeiro, cai para Windows-1252 (suporte a acentos PT/ES/FR)
fn decode_midi_text(bytes: &[u8]) -> String {
    // Tenta UTF-8 primeiro
    if let Ok(s) = std::str::from_utf8(bytes) {
        return s.to_string();
    }
    // Fallback: Windows-1252 (padrão de facto para KAR/MIDI antigos)
    let (decoded, _, _) = WINDOWS_1252.decode(bytes);
    decoded.into_owned()
}

/// Um fragmento (sílaba/palavra) com seu instante de início
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MidiSyllable {
    pub text: String,
    pub start_seconds: f64,
}

/// Uma linha de letra com todas as sílabas que a compõem
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MidiLyric {
    pub syllables: Vec<MidiSyllable>,
    pub start_seconds: f64,
    pub end_seconds: f64,
    // campos mantidos por compatibilidade
    pub text: String,
    pub start_tick: u64,
    pub end_tick: u64,
    pub duration_seconds: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynthTrack {
    pub name: String,
    pub note_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynthMetadata {
    pub title: String,
    pub artist: String,
    pub format: u16,
    pub tracks: Vec<SynthTrack>,
    pub total_ticks: u64,
    pub total_seconds: f64,
    pub lyrics: Vec<MidiLyric>,
}

pub struct SynthParser;

impl SynthParser {
    pub fn parse_file(path: &str) -> Result<SynthMetadata, String> {
        let data = fs::read(path).map_err(|e| format!("Failed to read file: {}", e))?;
        let smf = Smf::parse(&data).map_err(|e| format!("Failed to parse MIDI: {}", e))?;

        let mut title = String::new();
        let artist = String::new();
        let mut tracks = Vec::new();
        let mut total_ticks: u64 = 0;
        let mut tempo: u32 = 500000;

        let ticks_per_quarter: u64 = match smf.header.timing {
            midly::Timing::Metrical(t) => t.as_int() as u64,
            midly::Timing::Timecode(fps, subframes) => (fps.as_int() as u64) * (subframes as u64),
        };

        let mut raw_syllables: Vec<(u64, String)> = Vec::new();

        let format_type = match smf.header.format {
            midly::Format::SingleTrack => 0,
            midly::Format::Parallel => 1,
            midly::Format::Sequential => 2,
        };

        for (track_idx, track) in smf.tracks.iter().enumerate() {
            let mut current_tick: u64 = 0;
            let mut track_name = format!("Track {}", track_idx + 1);
            let mut note_count: u64 = 0;

            for event in track {
                current_tick += event.delta.as_int() as u64;

                match &event.kind {
                    midly::TrackEventKind::Midi { message, .. } => {
                        if let MidiMessage::NoteOn { .. } = message {
                            note_count += 1;
                        }
                    }
                    midly::TrackEventKind::Meta(msg) => match msg {
                        MetaMessage::TrackName(name) if !name.is_empty() => {
                            track_name = decode_midi_text(name);
                        }
                        MetaMessage::Text(text) | MetaMessage::Lyric(text) => {
                            let s = decode_midi_text(text);
                            if !s.trim().is_empty() {
                                raw_syllables.push((current_tick, s));
                            }
                        }
                        MetaMessage::Tempo(t) => { tempo = t.as_int(); }
                        _ => {}
                    },
                    _ => {}
                }

                if current_tick > total_ticks { total_ticks = current_tick; }
            }

            tracks.push(SynthTrack { name: track_name, note_count });
        }

        let sec_per_tick = (tempo as f64 / ticks_per_quarter as f64) / 1_000_000.0;
        let total_seconds = total_ticks as f64 * sec_per_tick;

        // --- Agrupar sílabas em linhas ---
        // Marcadores de quebra de linha no padrão KAR: \r, \n, /, \
        let mut lines: Vec<MidiLyric> = Vec::new();
        let mut cur_syls: Vec<MidiSyllable> = Vec::new();
        let mut line_start_tick: u64 = 0;
        let mut line_start_sec: f64 = 0.0;

        for (tick, raw_text) in &raw_syllables {
            let sec = *tick as f64 * sec_per_tick;

            let is_break = raw_text.starts_with('\r')
                || raw_text.starts_with('\n')
                || raw_text == "/"
                || raw_text == "\\";

            let clean: String = raw_text
                .trim_start_matches(['\r', '\n', '/', '\\'])
                .to_string();

            if is_break && !cur_syls.is_empty() {
                Self::push_line(&mut lines, &cur_syls, line_start_sec, sec, line_start_tick, *tick);
                cur_syls = Vec::new();
                if !clean.is_empty() {
                    cur_syls.push(MidiSyllable { text: clean, start_seconds: sec });
                    line_start_tick = *tick;
                    line_start_sec = sec;
                }
            } else if !clean.is_empty() {
                if cur_syls.is_empty() {
                    line_start_tick = *tick;
                    line_start_sec = sec;
                }
                cur_syls.push(MidiSyllable { text: clean, start_seconds: sec });
            }
        }
        if !cur_syls.is_empty() {
            let end = cur_syls.last().map(|s| s.start_seconds + 3.0).unwrap_or(total_seconds);
            Self::push_line(&mut lines, &cur_syls, line_start_sec, end, line_start_tick, total_ticks);
        }

        // Fallback: se sem marcadores de linha, agrupar por pausa > 1.5s
        if lines.len() <= 1 && !raw_syllables.is_empty() {
            lines.clear();
            let mut group: Vec<MidiSyllable> = Vec::new();
            let mut grp_start_sec = 0.0f64;
            let mut grp_start_tick = 0u64;

            for (i, (tick, text)) in raw_syllables.iter().enumerate() {
                let sec = *tick as f64 * sec_per_tick;
                let clean: String = text.trim_matches(['\r', '\n', '/', '\\']).to_string();
                if clean.is_empty() { continue; }

                let gap = if i > 0 {
                    sec - raw_syllables[i - 1].0 as f64 * sec_per_tick
                } else { 0.0 };

                if !group.is_empty() && gap > 1.5 {
                    let end = sec;
                    Self::push_line(&mut lines, &group, grp_start_sec, end, grp_start_tick, *tick);
                    group = Vec::new();
                }
                if group.is_empty() { grp_start_sec = sec; grp_start_tick = *tick; }
                group.push(MidiSyllable { text: clean, start_seconds: sec });
            }
            if !group.is_empty() {
                let end = group.last().map(|s| s.start_seconds + 3.0).unwrap_or(total_seconds);
                Self::push_line(&mut lines, &group, grp_start_sec, end, grp_start_tick, total_ticks);
            }
        }

        if title.is_empty() && tracks.len() > 1 {
            title = tracks[1].name.clone();
        }

        Ok(SynthMetadata { title, artist, format: format_type, tracks, total_ticks, total_seconds, lyrics: lines })
    }

    fn push_line(
        lines: &mut Vec<MidiLyric>,
        syls: &[MidiSyllable],
        start_sec: f64,
        end_sec: f64,
        start_tick: u64,
        end_tick: u64,
    ) {
        let text: String = syls.iter().map(|s| s.text.as_str()).collect();
        lines.push(MidiLyric {
            syllables: syls.to_vec(),
            start_seconds: start_sec,
            end_seconds: end_sec,
            text: text.trim().to_string(),
            start_tick,
            end_tick,
            duration_seconds: end_sec - start_sec,
        });
    }
}
