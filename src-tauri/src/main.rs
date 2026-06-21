#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod cdg_parser;

use std::sync::mpsc;
use std::thread;
use std::fs;
use std::time::Duration;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::Instant;
use std::io::BufReader;

use crate::cdg_parser::{CdgParser, CdgCommand};

// Enum de comandos para comunicar de forma segura com a thread de áudio nativa
enum AudioCommand {
    Play(String),
    #[allow(dead_code)]
    Stop,
}

// O AudioEngine atua como uma casca sobre o transmissor para a thread do Rodio
struct AudioEngine {
    tx: mpsc::Sender<AudioCommand>,
}

impl AudioEngine {
    fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        
        // Dispara uma thread nativa persistente para segurar o contexto do Rodio
        // Isso burla o fato de que o OutputStream do rodio não é thread-safe (não implementa Send).
        thread::spawn(move || {
            let (_stream, stream_handle) = rodio::OutputStream::try_default()
                .expect("Failed to get default audio output stream");
            
            let sink = rodio::Sink::try_new(&stream_handle).expect("Failed to create audio sink");
            
            // Fica bloqueado consumindo comandos (MPSC Rx)
            for cmd in rx {
                match cmd {
                    AudioCommand::Play(path) => {
                        println!("AudioEngine (Thread): Iniciando reprodução -> {}", path);
                        if let Ok(file) = std::fs::File::open(&path) {
                            if let Ok(source) = rodio::Decoder::new(BufReader::new(file)) {
                                sink.stop(); // Interrompe música anterior se houver
                                sink.append(source);
                                sink.play();
                            } else {
                                eprintln!("Falha ao decodificar arquivo de áudio: {}", path);
                            }
                        } else {
                            eprintln!("Falha ao abrir arquivo de áudio: {}", path);
                        }
                    }
                    AudioCommand::Stop => {
                        sink.stop();
                    }
                }
            }
        });

        Self { tx }
    }

    fn play_song(&self, path: String) {
        let _ = self.tx.send(AudioCommand::Play(path));
    }
}

// Estado compartilhado gerenciado pelo Tauri (async Mutex)
struct AppState {
    audio_engine: Arc<Mutex<AudioEngine>>,
}

#[derive(Clone, serde::Serialize)]
struct CdgBatchPayload {
    commands: Vec<CdgCommand>,
}

#[tauri::command]
async fn play_song(window: tauri::Window, state: tauri::State<'_, AppState>, path: String) -> Result<(), String> {
    // 1. Envia o comando de Play para a Thread de Áudio nativa
    let engine = state.audio_engine.lock().await;
    engine.play_song(path.clone());
    
    // 2. Tenta carregar os pacotes gráficos CD+G correspondentes
    let cdg_path = path.replace(".mp3", ".cdg").replace(".wav", ".cdg");
    
    let commands = if let Ok(data) = fs::read(&cdg_path) {
        let parser = CdgParser::new();
        parser.parse_file(&data)
    } else {
        println!("CDG file not found at {}, generating mock stream for UI test.", cdg_path);
        vec![
            CdgCommand::LoadColorTableLow { colors: [1056, 4000, 255, 0, 0, 0, 0, 0] },
            CdgCommand::MemoryPreset { color: 0, repeat: 0 },
            CdgCommand::TileBlockNormal { color0: 0, color1: 1, row: 8, col: 20, pixels: [0x3F, 0x21, 0x21, 0x21, 0x3F, 0, 0, 0, 0, 0, 0, 0] },
            CdgCommand::TileBlockNormal { color0: 0, color1: 2, row: 8, col: 21, pixels: [0x3F, 0x21, 0x21, 0x21, 0x3F, 0, 0, 0, 0, 0, 0, 0] },
            CdgCommand::TileBlockXor { color0: 0, color1: 1, row: 8, col: 22, pixels: [0x1E, 0x21, 0x21, 0x1E, 0x01, 0x01, 0x1E, 0, 0, 0, 0, 0] }
        ]
    };

    let total_commands = commands.len();
    
    // 3. MASTER CLOCK (Thread de Streaming e Sincronização Assíncrona)
    tokio::spawn(async move {
        // Taxa do CDG: Oficialmente 300 pacotes por segundo exatos.
        const PACKETS_PER_SEC: f64 = 300.0;
        let start_time = Instant::now();
        let mut interval = tokio::time::interval(Duration::from_millis(16)); // Poll de ~60Hz
        
        let mut last_processed_index = 0;

        loop {
            interval.tick().await; // Aguarda o próximo quadro gráfico
            
            // Relógio Matemático: calcula o exato milissegundo em que estamos e traduz para índice de pacote
            let elapsed_secs = start_time.elapsed().as_secs_f64();
            let target_index = (elapsed_secs * PACKETS_PER_SEC) as usize;
            
            if target_index > last_processed_index {
                let end_index = target_index.min(total_commands);
                if end_index > last_processed_index {
                    // Extrai apenas os pacotes correspondentes ao delta de tempo
                    let batch = commands[last_processed_index..end_index].to_vec();
                    
                    if let Err(e) = window.emit("cdg_batch", CdgBatchPayload { commands: batch }) {
                        eprintln!("Error emitting CDG batch: {}", e);
                        break;
                    }
                    
                    last_processed_index = end_index;
                }
            }

            if last_processed_index >= total_commands {
                println!("Fim da reprodução do CDG stream.");
                break; // Músic/Stream acabou
            }
        }
    });

    Ok(())
}

#[tauri::command]
async fn parse_cdg_file(path: String) -> Result<Vec<CdgCommand>, String> {
    let data = fs::read(&path).map_err(|e| e.to_string())?;
    let parser = CdgParser::new();
    Ok(parser.parse_file(&data))
}

fn main() {
    tauri::Builder::default()
        .manage(AppState {
            audio_engine: Arc::new(Mutex::new(AudioEngine::new())),
        })
        .invoke_handler(tauri::generate_handler![play_song, parse_cdg_file])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
