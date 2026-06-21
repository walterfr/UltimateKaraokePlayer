#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod cdg_parser;

use tauri::Manager;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::fs;
use std::time::Duration;
use crate::cdg_parser::{CdgParser, CdgCommand};

// Placeholder for the Audio Engine
struct AudioEngine {
    // will hold fluid-synth and rodio state
}

impl AudioEngine {
    fn new() -> Self {
        Self {}
    }
    fn play_song(&self, path: String) {
        println!("Playing song: {}", path);
        // Rodio implementation would go here in future
    }
}

// State shared across Tauri commands
struct AppState {
    audio_engine: Arc<Mutex<AudioEngine>>,
}

#[derive(Clone, serde::Serialize)]
struct CdgBatchPayload {
    commands: Vec<CdgCommand>,
}

#[tauri::command]
async fn play_song(window: tauri::Window, state: tauri::State<'_, AppState>, path: String) -> Result<(), String> {
    let engine = state.audio_engine.lock().await;
    engine.play_song(path.clone());
    
    // Tenta ler o arquivo CDG, ou gera um stream mockado se não existir/for arquivo de teste
    let cdg_path = path.replace(".mp3", ".cdg");
    
    let commands = if let Ok(data) = fs::read(&cdg_path) {
        let parser = CdgParser::new();
        parser.parse_file(&data)
    } else {
        // Mock stream if file doesn't exist
        vec![
            CdgCommand::LoadColorTableLow { colors: [1056, 4000, 255, 0, 0, 0, 0, 0] },
            CdgCommand::MemoryPreset { color: 0, repeat: 0 },
            CdgCommand::TileBlockNormal { color0: 0, color1: 1, row: 8, col: 20, pixels: [0x3F, 0x21, 0x21, 0x21, 0x3F, 0, 0, 0, 0, 0, 0, 0] },
            CdgCommand::TileBlockNormal { color0: 0, color1: 2, row: 8, col: 21, pixels: [0x3F, 0x21, 0x21, 0x21, 0x3F, 0, 0, 0, 0, 0, 0, 0] },
        ]
    };

    let total_commands = commands.len();
    
    // Spawn de uma task isolada para stremar os eventos ao frontend
    tokio::spawn(async move {
        // O CDG tem taxa de ~300 frames por segundo (1 comando por bloco de áudio)
        // Para não gargalar o IPC, enviaremos 5 comandos a cada 16.6ms (~60 FPS)
        let packets_per_batch = 5;
        let mut current_batch = Vec::with_capacity(packets_per_batch);
        let mut interval = tokio::time::interval(Duration::from_millis(16));
        
        for (i, cmd) in commands.into_iter().enumerate() {
            current_batch.push(cmd);
            
            if current_batch.len() >= packets_per_batch || i == total_commands - 1 {
                interval.tick().await; 
                let payload = CdgBatchPayload { commands: current_batch.clone() };
                if let Err(e) = window.emit("cdg_batch", payload) {
                    eprintln!("Error emitting CDG batch: {}", e);
                    break;
                }
                current_batch.clear();
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
