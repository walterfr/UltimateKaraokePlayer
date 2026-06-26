#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::Manager;

mod cdg_parser;
mod synth_engine;
mod video_engine;
mod tracker_engine;
mod legacy_engine;
mod ultrastar_engine;
mod library;
mod remote;
mod bass_engine;
pub mod star3_engine;

use std::sync::mpsc;
use std::thread;
use std::fs;
use std::time::Duration;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::Instant;
use std::io::BufReader;
use rodio::Source;

use crate::cdg_parser::{CdgParser, CdgCommand};
use crate::synth_engine::{SynthParser, SynthMetadata};
use crate::video_engine::{VideoEngine, VideoMetadata};
use crate::tracker_engine::{TrackerParser, TrackerMetadata};
use crate::legacy_engine::{LegacyParser, LegacyMetadata};
use crate::ultrastar_engine::{UltrastarParser, UltrastarMetadata};
use crate::library::Library;
use crate::library::{SongEntry, QueueEntry};
use rustysynth::{Synthesizer, SynthesizerSettings, SoundFont, MidiFile, MidiFileSequencer};
use xmrs::prelude::Module;
use xmrsplayer::xmrsplayer::XmrsPlayer;

struct MidiSource {
    pub sequencer: MidiFileSequencer,
    pub left_buf: Vec<f32>,
    pub right_buf: Vec<f32>,
    pub buf_idx: usize,
    pub debug_printed: bool,
}

impl MidiSource {
    fn new(midi_path: &str, sf2_path: &str, sample_rate: u32) -> Result<Self, String> {
        let mut sf2 = std::fs::File::open(sf2_path).map_err(|e| format!("SF2 error: {}", e))?;
        let sound_font = Arc::new(SoundFont::new(&mut sf2).map_err(|e| format!("SF2 parse error: {}", e))?);
        let settings = SynthesizerSettings::new(sample_rate as i32);
        let synthesizer = Synthesizer::new(&sound_font, &settings).map_err(|e| format!("Synth error: {}", e))?;
        
        let mut mid_file = std::fs::File::open(midi_path).map_err(|e| format!("MIDI error: {}", e))?;
        let midi_data = Arc::new(MidiFile::new(&mut mid_file).map_err(|e| format!("MIDI parse error: {}", e))?);
        
        let mut sequencer = MidiFileSequencer::new(synthesizer);
        sequencer.play(&midi_data, false);
        
        Ok(Self {
            sequencer,
            left_buf: vec![0.0; 1024],
            right_buf: vec![0.0; 1024],
            buf_idx: 1024 * 2,
            debug_printed: false,
        })
    }
}

impl Iterator for MidiSource {
    type Item = f32;
    fn next(&mut self) -> Option<Self::Item> {
        let max_samples = self.left_buf.len();
        let max_indices = max_samples * 2;
        
        if self.buf_idx >= max_indices {
            self.sequencer.render(&mut self.left_buf, &mut self.right_buf);
            self.buf_idx = 0;
            if self.sequencer.end_of_sequence() {
                return None;
            }
        }
        
        let sample = if self.buf_idx % 2 == 0 {
            self.left_buf[self.buf_idx / 2]
        } else {
            self.right_buf[self.buf_idx / 2]
        };
        
        self.buf_idx += 1;
        Some(sample)
    }
}

struct TrackerSource {
    player: XmrsPlayer<'static>,
    module_ptr: *mut Module,
}

unsafe impl Send for TrackerSource {}

impl TrackerSource {
    fn new(path: &str, sample_rate: u32) -> Result<Self, String> {
        let data = std::fs::read(path).map_err(|e| format!("IO error: {}", e))?;
        let module = Module::load(&data).map_err(|e| format!("XMRS error: {:?}", e))?;
        let boxed_module = Box::new(module);
        let module_ptr = Box::into_raw(boxed_module);
        let module_ref: &'static Module = unsafe { &*module_ptr };
        
        let mut player = XmrsPlayer::new(module_ref, sample_rate, 0);
        player.goto(0, 0, 0);
        Ok(Self {
            player,
            module_ptr,
        })
    }
}

impl Drop for TrackerSource {
    fn drop(&mut self) {
        unsafe {
            let _ = Box::from_raw(self.module_ptr);
        }
    }
}

impl Iterator for TrackerSource {
    type Item = f32;
    fn next(&mut self) -> Option<Self::Item> {
        self.player.next().map(|s| s as f32 / 32768.0)
    }
}

fn build_cpal_stream<I>(device: &cpal::Device, mut source: I, volume: Arc<std::sync::atomic::AtomicU32>) -> Result<cpal::Stream, String>
where
    I: Iterator<Item = f32> + Send + 'static,
{
    use cpal::traits::{DeviceTrait, StreamTrait};
    let config = device.default_output_config().map_err(|e| e.to_string())?;
    let channels = config.channels() as usize;

    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => device.build_output_stream(
            &config.config(),
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                let vol = f32::from_bits(volume.load(std::sync::atomic::Ordering::Relaxed));
                for frame in data.chunks_mut(channels) {
                    let l = source.next().unwrap_or(0.0) * vol;
                    let r = source.next().unwrap_or(0.0) * vol;
                    if channels >= 1 { frame[0] = l; }
                    if channels >= 2 { frame[1] = r; }
                }
            },
            |err| eprintln!("[CPAL] error: {}", err),
            None
        ).map_err(|e| e.to_string())?,
        cpal::SampleFormat::I16 => device.build_output_stream(
            &config.config(),
            move |data: &mut [i16], _: &cpal::OutputCallbackInfo| {
                let vol = f32::from_bits(volume.load(std::sync::atomic::Ordering::Relaxed));
                for frame in data.chunks_mut(channels) {
                    let l = (source.next().unwrap_or(0.0) * vol * 32767.0) as i16;
                    let r = (source.next().unwrap_or(0.0) * vol * 32767.0) as i16;
                    if channels >= 1 { frame[0] = l; }
                    if channels >= 2 { frame[1] = r; }
                }
            },
            |err| eprintln!("[CPAL] error: {}", err),
            None
        ).map_err(|e| e.to_string())?,
        _ => return Err("Unsupported sample format by CPAL".to_string()),
    };

    stream.play().map_err(|e| e.to_string())?;
    Ok(stream)
}

// Enum de comandos para comunicar de forma segura com a thread de Ã¡udio nativa
enum AudioCommand {
    Play(String),
    Pause,
    Resume,
    Stop,
    SetVolume(f32),
    Seek(f64),
    SetOutputDevice(String),
}

// O AudioEngine atua como uma casca sobre o transmissor para a thread do Rodio
struct AudioEngine {
    tx: mpsc::Sender<AudioCommand>,
}


impl AudioEngine {
    fn new(sf2_path: Option<std::path::PathBuf>) -> Self {
        let (tx, rx) = mpsc::channel();
        
        thread::spawn(move || {
            use cpal::traits::{HostTrait, DeviceTrait, StreamTrait};
            use std::sync::atomic::AtomicU32;
            if let Err(e) = crate::bass_engine::BassEngine::init() {
                eprintln!("[BASS INIT ERROR] {}", e);
            }
            let host = cpal::default_host();
            let mut current_device = host.default_output_device();
            
            let mut current_stream: Option<rodio::OutputStream> = None;
            let mut current_handle: Option<rodio::OutputStreamHandle> = None;
            let mut sink: Option<rodio::Sink> = None;
            
            let mut current_cpal_stream: Option<cpal::Stream> = None;
            let mut current_bass_handle: Option<crate::bass_engine::HSTREAM> = None;
            
            let current_volume_atomic = Arc::new(AtomicU32::new(1.0f32.to_bits()));
            let mut current_volume: f32 = 1.0;
            let mut current_path: Option<String> = None;
            let mut is_midi: bool = false;
            let mut is_tracker: bool = false;
            let mut is_legacy: bool = false;
            
            match rodio::OutputStream::try_default() {
                Ok((stream, handle)) => {
                    current_stream = Some(stream);
                    current_handle = Some(handle);
                    println!("[AUDIO] Inicializado com dispositivo padrÃ£o");
                }
                Err(e) => eprintln!("[AUDIO] Erro ao abrir dispositivo padrÃ£o: {}", e),
            }
            
            for cmd in rx {
                match cmd {
                    AudioCommand::SetOutputDevice(name) => {
                        println!("[AUDIO] Trocando output para: {}", name);
                        
                        if let Some(old_stream) = current_cpal_stream.take() {
                            drop(old_stream);
                        }
                        if let Some(old_sink) = sink.take() {
                            old_sink.stop();
                            drop(old_sink);
                        }
                        drop(current_handle.take());
                        drop(current_stream.take());
                        
                        if name.contains("(PadrÃ£o)") {
                            current_device = host.default_output_device();
                            if let Ok((stream, handle)) = rodio::OutputStream::try_default() {
                                current_stream = Some(stream);
                                current_handle = Some(handle);
                            }
                        } else {
                            let found = host.output_devices().ok().and_then(|devices| {
                                devices.into_iter().find(|d| d.name().map(|n| n == name).unwrap_or(false))
                            });
                            
                            match found {
                                Some(device) => {
                                    if let Ok((stream, handle)) = rodio::OutputStream::try_from_device(&device) {
                                        current_stream = Some(stream);
                                        current_handle = Some(handle);
                                    }
                                    current_device = Some(device);
                                }
                                None => {
                                    current_device = host.default_output_device();
                                    if let Ok((stream, handle)) = rodio::OutputStream::try_default() {
                                        current_stream = Some(stream);
                                        current_handle = Some(handle);
                                    }
                                }
                            }
                        }
                    }
                    AudioCommand::Play(path) => {
                        println!("AudioEngine: Iniciando reproduÃ§Ã£o -> {}", path);
                        
                        if let Some(old_stream) = current_cpal_stream.take() { drop(old_stream); }
                        if let Some(old_sink) = sink.take() { old_sink.stop(); drop(old_sink); }
                        if let Some(h) = current_bass_handle.take() {
                            crate::bass_engine::BassEngine::stop(h).unwrap_or_default();
                        }
                        
                        let ext = path.split('.').last().unwrap_or("").to_lowercase();
                        let is_star3 = ext == "st3";
                        is_midi = ext == "mid" || ext == "kar" || is_star3;
                        is_tracker = ext == "mod" || ext == "s3m" || ext == "xm" || ext == "it" || ext == "zip";
                        is_legacy = ext == "mk1" || ext == "kara";
                        
                        let mut final_path = path.clone();
                        if is_star3 {
                            if let Ok(mid_path) = crate::star3_engine::Star3Parser::decode_to_midi(&path) {
                                final_path = mid_path;
                            } else {
                                eprintln!("[ST3] Erro ao decodificar");
                            }
                        }

                        // Formatos Legacy proprietÃ¡rios (MK1/KARA): decodificar para MIDI
                        // e rotear para RustySynth, bypassando o BASS que nÃ£o os reconhece.
                        if is_legacy {
                            let decoded_result = if ext == "mk1" {
                                crate::legacy_engine::LegacyParser::decode_mk1_to_midi(&path)
                            } else {
                                // .kara: pode ser MIDI puro ou MK1-encapsulado
                                crate::legacy_engine::LegacyParser::decode_kara_to_midi(&path)
                            };
                            match decoded_result {
                                Ok(mid_path) => {
                                    println!("[LEGACY] Decodificado para MIDI: {}", mid_path);
                                    final_path = mid_path;
                                    is_midi = true;
                                    is_legacy = false; // evita fallback para BASS
                                }
                                Err(e) => {
                                    eprintln!("[LEGACY] Falha ao decodificar para MIDI: {}", e);
                                    // is_legacy permanece true â†’ BASS como Ãºltima tentativa
                                }
                            }
                        }

                        if is_tracker || is_legacy {
                            if let Ok(handle) = crate::bass_engine::BassEngine::load_auto(&path) {
                                crate::bass_engine::BassEngine::set_volume(handle, current_volume).unwrap_or_default();
                                crate::bass_engine::BassEngine::play(handle).unwrap_or_default();
                                current_bass_handle = Some(handle);
                            } else {
                                eprintln!("[BASS] Erro ao carregar tracker/legacy");
                            }
                        } else if is_midi {
                            if let Some(ref device) = current_device {
                                if let Ok(config) = device.default_output_config() {
                                    if let Some(ref sf2_path) = sf2_path {
                                        let sf2_str = sf2_path.to_string_lossy().to_string();
                                        match MidiSource::new(&final_path, &sf2_str, config.sample_rate().0) {
                                            Ok(source) => {
                                                if let Ok(stream) = build_cpal_stream(device, source, current_volume_atomic.clone()) {
                                                    current_cpal_stream = Some(stream);
                                                }
                                            }
                                            Err(e) => eprintln!("[MIDI] Erro: {}", e),
                                        }
                                    }
                                }
                            }
                        } else {
                            if current_handle.is_none() {
                                if let Ok((stream, handle)) = rodio::OutputStream::try_default() {
                                    current_stream = Some(stream);
                                    current_handle = Some(handle);
                                }
                            }
                            if let Some(ref h) = current_handle {
                                if let Ok(new_sink) = rodio::Sink::try_new(h) {
                                    new_sink.set_volume(current_volume);
                                    if let Ok(file) = std::fs::File::open(&path) {
                                        if let Ok(source) = rodio::Decoder::new(BufReader::new(file)) {
                                            new_sink.append(source);
                                            new_sink.play();
                                            sink = Some(new_sink);
                                        }
                                    }
                                }
                            }
                        }
                        current_path = Some(final_path);
                    }
                    AudioCommand::Pause => {
                        if let Some(ref s) = sink { s.pause(); }
                        if let Some(ref s) = current_cpal_stream { s.pause().unwrap_or(()); }
                        if let Some(h) = current_bass_handle {
                            crate::bass_engine::BassEngine::pause(h).unwrap_or_default();
                        }
                    }
                    AudioCommand::Resume => {
                        if let Some(ref s) = sink { s.play(); }
                        if let Some(ref s) = current_cpal_stream { s.play().unwrap_or(()); }
                        if let Some(h) = current_bass_handle {
                            crate::bass_engine::BassEngine::play(h).unwrap_or_default();
                        }
                    }
                    AudioCommand::Stop => {
                        if let Some(old_stream) = current_cpal_stream.take() { drop(old_stream); }
                        if let Some(old_sink) = sink.take() { old_sink.stop(); drop(old_sink); }
                        if let Some(h) = current_bass_handle.take() {
                            crate::bass_engine::BassEngine::stop(h).unwrap_or_default();
                        }
                        current_path = None;
                        is_midi = false;
                        is_tracker = false;
                        is_legacy = false;
                    }
                    AudioCommand::SetVolume(vol) => {
                        current_volume = vol;
                        current_volume_atomic.store(vol.to_bits(), std::sync::atomic::Ordering::Relaxed);
                        if let Some(ref s) = sink { s.set_volume(vol); }
                        if let Some(h) = current_bass_handle {
                            crate::bass_engine::BassEngine::set_volume(h, vol).unwrap_or_default();
                        }
                    }
                    AudioCommand::Seek(pos) => {
                        if is_tracker || is_legacy {
                            if let Some(h) = current_bass_handle {
                                crate::bass_engine::BassEngine::seek(h, pos).unwrap_or_default();
                            }
                        } else if is_midi {
                            if let Some(ref path) = current_path {
                                if let Some(old_stream) = current_cpal_stream.take() { drop(old_stream); }
                                if let Some(ref sf2_path) = sf2_path {
                                    if let Some(ref device) = current_device {
                                        if let Ok(config) = device.default_output_config() {
                                            let sample_rate = config.sample_rate().0;
                                            let sf2_str = sf2_path.to_string_lossy().to_string();
                                            // TODO: seeking is_star3 requires keeping the temp path around, but this might just restart the track if it was deleted.
                                            if let Ok(mut source) = MidiSource::new(path, &sf2_str, sample_rate) {
                                                let samples_to_skip = (pos * sample_rate as f64) as usize;
                                                let mut skipped = 0usize;
                                                let buf_size = source.left_buf.len();
                                                while skipped < samples_to_skip && !source.sequencer.end_of_sequence() {
                                                    source.sequencer.render(&mut source.left_buf, &mut source.right_buf);
                                                    skipped += buf_size;
                                                }
                                                source.buf_idx = buf_size * 2;
                                                if let Ok(stream) = build_cpal_stream(device, source, current_volume_atomic.clone()) {
                                                    current_cpal_stream = Some(stream);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        } else {
                            if let Some(ref path) = current_path {
                                if let Some(old_sink) = sink.take() { old_sink.stop(); }
                                if let Some(ref h) = current_handle {
                                    if let Ok(new_sink) = rodio::Sink::try_new(h) {
                                        new_sink.set_volume(current_volume);
                                        if let Ok(file) = std::fs::File::open(path) {
                                            if let Ok(source) = rodio::Decoder::new(BufReader::new(file)) {
                                                let skip_dur = std::time::Duration::from_secs_f64(pos);
                                                use rodio::Source;
                                                let skipped = source.skip_duration(skip_dur);
                                                new_sink.append(skipped);
                                                new_sink.play();
                                                sink = Some(new_sink);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });

        Self { tx }
    }
    fn play_song(&self, path: String) {
        let _ = self.tx.send(AudioCommand::Play(path));
    }

    fn pause(&self) {
        let _ = self.tx.send(AudioCommand::Pause);
    }

    fn resume(&self) {
        let _ = self.tx.send(AudioCommand::Resume);
    }

    fn stop(&self) {
        let _ = self.tx.send(AudioCommand::Stop);
    }

    fn set_volume(&self, vol: f32) {
        let _ = self.tx.send(AudioCommand::SetVolume(vol));
    }

    fn seek_to(&self, pos: f64) {
        let _ = self.tx.send(AudioCommand::Seek(pos));
    }

    fn set_output_device(&self, name: String) {
        let _ = self.tx.send(AudioCommand::SetOutputDevice(name));
    }
}

use std::sync::atomic::{AtomicUsize, Ordering};

// Estado compartilhado gerenciado pelo Tauri (async Mutex)
struct AppState {
    audio_engine: Arc<Mutex<AudioEngine>>,
    library: Arc<Mutex<Library>>,
    current_song_id: Arc<AtomicUsize>,
}

#[derive(Clone, serde::Serialize)]
struct CdgBatchPayload {
    commands: Vec<CdgCommand>,
    current_time: f64,
    total_duration: f64,
}

#[tauri::command]
async fn play_song(window: tauri::Window, state: tauri::State<'_, AppState>, path: String) -> Result<(), String> {
    // 1. Envia o comando de Play para a Thread de Ãudio nativa
    let engine = state.audio_engine.lock().await;
    engine.play_song(path.clone());
    
    // Incrementa e captura o ID da mÃºsica atual para cancelamento
    let song_id = state.current_song_id.fetch_add(1, Ordering::SeqCst) + 1;
    let song_id_tracker = state.current_song_id.clone();
    
    // Marca o instante exato em que o Ã¡udio comeÃ§ou
    let start_time = Instant::now();

    // 2. Tenta carregar os pacotes grÃ¡ficos CD+G correspondentes
    let cdg_path = path
        .rsplit_once('.')
        .map(|(base, _)| format!("{}.cdg", base))
        .unwrap_or_default();

    let (commands, total_packets) = if let Ok(data) = fs::read(&cdg_path) {
        let parser = CdgParser::new();
        let parsed = parser.parse_file_with_indices(&data);
        let total = data.len() / 24;
        println!("CDG file found: {} ({} packets, {} commands)", cdg_path, total, parsed.len());
        (parsed, total)
    } else {
        println!("CDG file not found at {}, generating mock stream for UI test.", cdg_path);
        (vec![
            (0, CdgCommand::LoadColorTableLow { colors: [1056, 4000, 255, 0, 0, 0, 0, 0] }),
            (1, CdgCommand::MemoryPreset { color: 0, repeat: 0 }),
            (2, CdgCommand::TileBlockNormal { color0: 0, color1: 1, row: 8, col: 20, pixels: [0x3F, 0x21, 0x21, 0x21, 0x3F, 0, 0, 0, 0, 0, 0, 0] }),
            (3, CdgCommand::TileBlockNormal { color0: 0, color1: 2, row: 8, col: 21, pixels: [0x3F, 0x21, 0x21, 0x21, 0x3F, 0, 0, 0, 0, 0, 0, 0] }),
            (4, CdgCommand::TileBlockXor { color0: 0, color1: 1, row: 8, col: 22, pixels: [0x1E, 0x21, 0x21, 0x1E, 0x01, 0x01, 0x1E, 0, 0, 0, 0, 0] }),
        ], 300 * 30)
    };

    let total_duration = total_packets as f64 / 300.0;
    
    // 3. MASTER CLOCK (Thread de Streaming e SincronizaÃ§Ã£o AssÃ­ncrona)
    tokio::spawn(async move {
        // Aguarda meio segundo para garantir que o componente React <CdgCanvas /> foi montado e registrou o `listen` IPC
        tokio::time::sleep(Duration::from_millis(500)).await;

        const PACKETS_PER_SEC: f64 = 300.0;
        let mut interval = tokio::time::interval(Duration::from_millis(16));
        let mut last_sent_idx: i64 = -1;

        loop {
            // Verifica se o usuÃ¡rio apertou Stop ou deu Play em outra mÃºsica
            if song_id_tracker.load(Ordering::SeqCst) != song_id {
                println!("Master clock abortado (nova mÃºsica ou stop).");
                break;
            }

            interval.tick().await;
            // O elapsed() compensa o sleep de 500ms porque o start_time foi marcado ANTES do sleep!
            let elapsed_secs = start_time.elapsed().as_secs_f64();
            let target_packet = (elapsed_secs * PACKETS_PER_SEC) as i64;

            if target_packet > last_sent_idx {
                let mut batch = Vec::new();
                for (packet_idx, cmd) in &commands {
                    let p = *packet_idx as i64;
                    if p > last_sent_idx && p <= target_packet {
                        batch.push(cmd.clone());
                    }
                }
                if !batch.is_empty() {
                    if let Err(e) = window.emit("cdg_batch", CdgBatchPayload {
                        commands: batch,
                        current_time: elapsed_secs,
                        total_duration,
                    }) {
                        eprintln!("Error emitting CDG batch: {}", e);
                        break;
                    }
                }
                last_sent_idx = target_packet;
            }

            if last_sent_idx >= total_packets as i64 {
                println!("Fim da reproduÃ§Ã£o do CDG stream.");
                break;
            }
        }
    });

    Ok(())
}

#[tauri::command]
async fn pause_audio(state: tauri::State<'_, AppState>) -> Result<(), String> {
    state.audio_engine.lock().await.pause();
    Ok(())
}

#[tauri::command]
async fn resume_audio(state: tauri::State<'_, AppState>) -> Result<(), String> {
    state.audio_engine.lock().await.resume();
    Ok(())
}

#[tauri::command]
async fn stop_audio(state: tauri::State<'_, AppState>) -> Result<(), String> {
    state.current_song_id.fetch_add(1, Ordering::SeqCst);
    state.audio_engine.lock().await.stop();
    Ok(())
}

#[tauri::command]
async fn set_volume(state: tauri::State<'_, AppState>, volume: f32) -> Result<(), String> {
    state.audio_engine.lock().await.set_volume(volume);
    Ok(())
}

#[tauri::command]
async fn seek_to(state: tauri::State<'_, AppState>, position: f64) -> Result<(), String> {
    state.audio_engine.lock().await.seek_to(position);
    Ok(())
}

#[tauri::command]
async fn parse_cdg_file(path: String) -> Result<Vec<CdgCommand>, String> {
    let data = fs::read(&path).map_err(|e| e.to_string())?;
    let parser = CdgParser::new();
    Ok(parser.parse_file(&data))
}

#[tauri::command]
async fn parse_midi_file(path: String) -> Result<SynthMetadata, String> {
    println!("[DEBUG] Tentando fazer parse do arquivo MIDI: {}", path);
    let mut final_path = path.clone();
    let ext = path.split('.').last().unwrap_or("").to_lowercase();
    if ext == "st3" {
        if let Ok(mid_path) = crate::star3_engine::Star3Parser::decode_to_midi(&path) {
            final_path = mid_path;
        }
    } else if ext == "mk1" {
        if let Ok(mid_path) = crate::legacy_engine::LegacyParser::decode_mk1_to_midi(&path) {
            final_path = mid_path;
        }
    } else if ext == "kara" {
        if let Ok(mid_path) = crate::legacy_engine::LegacyParser::decode_kara_to_midi(&path) {
            final_path = mid_path;
        }
    }
    match SynthParser::parse_file(&final_path) {
        Ok(res) => {
            println!("[DEBUG] Parse MIDI bem-sucedido.");
            Ok(res)
        }
        Err(e) => {
            eprintln!("[DEBUG] Falha no parse MIDI: {}", e);
            Err(e)
        }
    }
}

#[tauri::command]
async fn parse_video_file(path: String) -> Result<VideoMetadata, String> {
    VideoEngine::parse_subtitles(&path)
}

#[tauri::command]
async fn parse_tracker_file(path: String) -> Result<TrackerMetadata, String> {
    println!("[DEBUG] Tentando fazer parse do arquivo TRACKER: {}", path);
    match TrackerParser::parse_file(&path) {
        Ok(res) => {
            println!("[DEBUG] Parse TRACKER bem-sucedido.");
            Ok(res)
        }
        Err(e) => {
            eprintln!("[DEBUG] Falha no parse TRACKER: {}", e);
            Err(e)
        }
    }
}

#[tauri::command]
async fn parse_legacy_file(path: String) -> Result<LegacyMetadata, String> {
    LegacyParser::parse_file(&path)
}

#[tauri::command]
async fn parse_ultrastar_file(path: String) -> Result<UltrastarMetadata, String> {
    UltrastarParser::parse_file(&path)
}

// Library commands


#[tauri::command]
async fn scan_library(state: tauri::State<'_, AppState>, path: String, engine: Option<String>) -> Result<serde_json::Value, String> {
    let lib = state.library.lock().await;
    match lib.scan_directory(&path, engine).await {
        Ok(count) => Ok(serde_json::json!({ "count": count })),
        Err(e) => Err(e),
    }
}

#[tauri::command]
async fn search_songs(state: tauri::State<'_, AppState>, query: String, sort_by: Option<String>) -> Result<Vec<SongEntry>, String> {
    let lib = state.library.lock().await;
    let sort = sort_by.unwrap_or_else(|| "title".to_string());
    lib.search_songs(&query, &sort).await
}

#[tauri::command]
async fn enqueue_song(state: tauri::State<'_, AppState>, song_id: i64, requested_by: String) -> Result<i64, String> {
    let lib = state.library.lock().await;
    lib.enqueue(song_id, &requested_by).await
}

#[tauri::command]
async fn get_queue(state: tauri::State<'_, AppState>) -> Result<Vec<QueueEntry>, String> {
    let lib = state.library.lock().await;
    lib.get_queue().await
}

#[tauri::command]
async fn remove_from_queue(state: tauri::State<'_, AppState>, queue_id: i64) -> Result<(), String> {
    let lib = state.library.lock().await;
    lib.remove_from_queue(queue_id).await
}

#[tauri::command]
async fn clear_library(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let lib = state.library.lock().await;
    lib.clear_library().await
}

#[tauri::command]
async fn clear_queue(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let lib = state.library.lock().await;
    lib.clear_queue().await
}

#[tauri::command]
async fn save_setting(state: tauri::State<'_, AppState>, key: String, value: String) -> Result<(), String> {
    let lib = state.library.lock().await;
    lib.save_setting(&key, &value).await
}

#[tauri::command]
async fn get_all_settings(state: tauri::State<'_, AppState>) -> Result<std::collections::HashMap<String, String>, String> {
    let lib = state.library.lock().await;
    lib.get_all_settings().await
}

#[tauri::command]
async fn get_songs_count(state: tauri::State<'_, AppState>) -> Result<i64, String> {
    let lib = state.library.lock().await;
    lib.get_songs_count().await
}

#[tauri::command]
fn list_output_devices() -> Result<Vec<String>, String> {
    use cpal::traits::{HostTrait, DeviceTrait};
    let host = cpal::default_host();
    let mut names = Vec::new();
    
    // Add default device first
    if let Some(default) = host.default_output_device() {
        if let Ok(name) = default.name() {
            names.push(format!("â­ {} (PadrÃ£o)", name));
        }
    }
    
    // Add all other devices
    if let Ok(devices) = host.output_devices() {
        for device in devices {
            if let Ok(name) = device.name() {
                if !names.iter().any(|n: &String| n.contains(&name)) {
                    names.push(name);
                }
            }
        }
    }
    
    Ok(names)
}

#[tauri::command]
async fn set_output_device(state: tauri::State<'_, AppState>, device_name: String) -> Result<(), String> {
    let engine = state.audio_engine.lock().await;
    engine.set_output_device(device_name);
    Ok(())
}

fn main() {
    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");

    // Usar diretÃ³rio fora de src-tauri/ para evitar que o watcher do Tauri
    // reinicie o app toda vez que o banco de dados for escrito durante o scan.
    let db_path = {
        let mut path = std::env::temp_dir();
        path.push("ultimate_karaoke_player");
        std::fs::create_dir_all(&path).ok();
        path.push("karaoke.db");
        format!("sqlite:{}?mode=rwc", path.display())
    };

    println!("[DB] Usando banco de dados em: {}", db_path);

    let library = rt.block_on(async {
        Library::new(&db_path).await
            .expect("Failed to initialize library database")
    });

    let library_arc = Arc::new(Mutex::new(library));

    tauri::Builder::default()
        .setup({
            let library_arc = library_arc.clone();
            move |app| {
                let handle = app.handle();
                let state = remote::RemoteState {
                    library: library_arc.clone(),
                    app_handle: handle,
                };
                
                tauri::async_runtime::spawn(async move {
                    remote::start_server(state).await;
                });
                
                let sf2_path = app.path_resolver()
                    .resolve_resource("TimGM6mb.sf2");
                
                app.manage(AppState {
                    audio_engine: Arc::new(Mutex::new(AudioEngine::new(sf2_path))),
                    library: library_arc,
                    current_song_id: Arc::new(AtomicUsize::new(0)),
                });
                
                Ok(())
            }
        })
        .invoke_handler(tauri::generate_handler![
            play_song, parse_cdg_file, parse_midi_file, parse_video_file,
            parse_tracker_file, parse_legacy_file, parse_ultrastar_file,
            stop_audio, pause_audio, resume_audio, set_volume, seek_to,
            scan_library, search_songs, get_songs_count, clear_library,
            enqueue_song, get_queue, remove_from_queue, clear_queue,
            save_setting, get_all_settings,
            list_output_devices, set_output_device
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
