use serde::{Serialize, Deserialize};

/// Constantes para comandos CD+G
pub const CDG_COMMAND_MASK: u8 = 0x09;

/// Códigos de Instrução CD+G
pub const INST_MEMORY_PRESET: u8 = 1;
pub const INST_BORDER_PRESET: u8 = 2;
pub const INST_TILE_BLOCK_NORMAL: u8 = 6;
pub const INST_SCROLL_PRESET: u8 = 20;
pub const INST_SCROLL_COPY: u8 = 24;
pub const INST_DEFINE_TRANSPARENT_COLOR: u8 = 28;
pub const INST_LOAD_COLOR_TABLE_LOW: u8 = 30;
pub const INST_LOAD_COLOR_TABLE_HIGH: u8 = 31;
pub const INST_TILE_BLOCK_XOR: u8 = 38;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum CdgCommand {
    MemoryPreset { color: u8, repeat: u8 },
    BorderPreset { color: u8 },
    TileBlockNormal { color0: u8, color1: u8, row: u8, col: u8, pixels: [u8; 12] },
    ScrollPreset { color: u8, h_cmd: u8, h_offset: u8, v_cmd: u8, v_offset: u8 },
    ScrollCopy { h_cmd: u8, h_offset: u8, v_cmd: u8, v_offset: u8 },
    DefineTransparentColor { color: u8 },
    LoadColorTableLow { colors: [u16; 8] },
    LoadColorTableHigh { colors: [u16; 8] },
    TileBlockXor { color0: u8, color1: u8, row: u8, col: u8, pixels: [u8; 12] },
}

pub struct CdgParser;

impl Default for CdgParser {
    fn default() -> Self {
        Self::new()
    }
}

impl CdgParser {
    pub fn new() -> Self {
        Self
    }

    pub fn parse_packet(&self, packet: &[u8]) -> Option<CdgCommand> {
        if packet.len() < 24 {
            return None;
        }

        let command = packet[0] & 0x3F;
        if command != CDG_COMMAND_MASK {
            return None; // Não é um comando CDG
        }

        let instruction = packet[1] & 0x3F;
        let data = &packet[4..20]; // 16 bytes úteis de dados

        match instruction {
            INST_MEMORY_PRESET => {
                Some(CdgCommand::MemoryPreset {
                    color: data[0] & 0x0F,
                    repeat: data[1] & 0x0F,
                })
            }
            INST_BORDER_PRESET => {
                Some(CdgCommand::BorderPreset {
                    color: data[0] & 0x0F,
                })
            }
            INST_TILE_BLOCK_NORMAL => {
                let mut pixels = [0u8; 12];
                pixels.copy_from_slice(&data[4..16]);
                Some(CdgCommand::TileBlockNormal {
                    color0: data[0] & 0x0F,
                    color1: data[1] & 0x0F,
                    row: data[2] & 0x1F,
                    col: data[3] & 0x3F,
                    pixels,
                })
            }
            INST_SCROLL_PRESET => {
                Some(CdgCommand::ScrollPreset {
                    color: data[0] & 0x0F,
                    h_cmd: (data[1] >> 4) & 0x03,
                    h_offset: data[1] & 0x0F,
                    v_cmd: (data[2] >> 4) & 0x03,
                    v_offset: data[2] & 0x0F,
                })
            }
            INST_SCROLL_COPY => {
                Some(CdgCommand::ScrollCopy {
                    h_cmd: (data[1] >> 4) & 0x03,
                    h_offset: data[1] & 0x0F,
                    v_cmd: (data[2] >> 4) & 0x03,
                    v_offset: data[2] & 0x0F,
                })
            }
            INST_DEFINE_TRANSPARENT_COLOR => {
                Some(CdgCommand::DefineTransparentColor {
                    color: data[0] & 0x0F,
                })
            }
            INST_LOAD_COLOR_TABLE_LOW => {
                Some(CdgCommand::LoadColorTableLow {
                    colors: Self::parse_colors(data),
                })
            }
            INST_LOAD_COLOR_TABLE_HIGH => {
                Some(CdgCommand::LoadColorTableHigh {
                    colors: Self::parse_colors(data),
                })
            }
            INST_TILE_BLOCK_XOR => {
                let mut pixels = [0u8; 12];
                pixels.copy_from_slice(&data[4..16]);
                Some(CdgCommand::TileBlockXor {
                    color0: data[0] & 0x0F,
                    color1: data[1] & 0x0F,
                    row: data[2] & 0x1F,
                    col: data[3] & 0x3F,
                    pixels,
                })
            }
            _ => None,
        }
    }

    pub fn parse_file(&self, data: &[u8]) -> Vec<CdgCommand> {
        let mut commands = Vec::new();
        for chunk in data.chunks_exact(24) {
            if let Some(cmd) = self.parse_packet(chunk) {
                commands.push(cmd);
            }
        }
        commands
    }

    fn parse_colors(data: &[u8]) -> [u16; 8] {
        let mut colors = [0u16; 8];
        for i in 0..8 {
            let high = data[i * 2] & 0x3F;
            let low = data[i * 2 + 1] & 0x3F;
            // Combina os 6 bits altos com os 6 bits baixos resultando em uma cor 12-bits RGB
            colors[i] = ((high as u16) << 6) | (low as u16);
        }
        colors
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_preset() {
        let mut packet = [0u8; 24];
        packet[0] = CDG_COMMAND_MASK;
        packet[1] = INST_MEMORY_PRESET;
        packet[4] = 0x07; // color 7
        packet[5] = 0x0A; // repeat 10

        let parser = CdgParser::new();
        let cmd = parser.parse_packet(&packet).unwrap();
        
        assert_eq!(cmd, CdgCommand::MemoryPreset { color: 7, repeat: 10 });
    }

    #[test]
    fn test_load_color_table_low() {
        let mut packet = [0u8; 24];
        packet[0] = CDG_COMMAND_MASK;
        packet[1] = INST_LOAD_COLOR_TABLE_LOW;
        
        // Cor 0: high=0x10, low=0x20 -> (16 << 6) | 32 = 1024 + 32 = 1056
        packet[4] = 0x10;
        packet[5] = 0x20;

        let parser = CdgParser::new();
        let cmd = parser.parse_packet(&packet).unwrap();
        
        match cmd {
            CdgCommand::LoadColorTableLow { colors } => {
                assert_eq!(colors[0], 1056);
                assert_eq!(colors[1], 0); // restantes zero
            },
            _ => panic!("Esperado LoadColorTableLow"),
        }
    }

    #[test]
    fn test_not_cdg_command() {
        let mut packet = [0u8; 24];
        packet[0] = 0x0A; // Máscara inválida
        let parser = CdgParser::new();
        assert!(parser.parse_packet(&packet).is_none());
    }
}
