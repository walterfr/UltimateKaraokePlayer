use std::fs;

pub struct Star3Parser;

#[derive(Debug)]
pub struct Star3Metadata {
    pub title: String,
    pub artist: String,
}

impl Star3Parser {
    pub fn is_star3(data: &[u8]) -> bool {
        data.starts_with(b"STAR DATA")
    }

    pub fn parse_file(path: &str) -> Result<Star3Metadata, String> {
        let data = fs::read(path).map_err(|e| format!("Failed to read {}: {}", path, e))?;
        
        if !Self::is_star3(&data) {
            return Err("Not a STAR DATA V3.50 file".to_string());
        }

        // Título está no offset 0x14 (20)
        let title = Self::read_string(&data, 20);
        
        // Artista parece ficar fixamente no offset 0x84 (132) no V3.50
        let artist = Self::read_string(&data, 132);

        Ok(Star3Metadata {
            title,
            artist,
        })
    }

    fn read_string(data: &[u8], offset: usize) -> String {
        if offset >= data.len() {
            return String::new();
        }
        let end = data[offset..].iter().position(|&b| b == 0).unwrap_or(data.len() - offset);
        let slice = &data[offset..offset + end];
        
        // st3 usa latin1 na maioria das vezes, vamos decodificar de forma tolerante (lossy se fosse utf8 puro)
        // Convertendo latin1 -> char
        slice.iter().map(|&c| c as char).collect()
    }

    /// Tenta decodificar de forma tolerante os eventos do ST3 e convertê-los para um arquivo MIDI padrão (.mid)
    pub fn decode_to_midi(st3_path: &str) -> Result<String, String> {
        let data = fs::read(st3_path).map_err(|e| format!("Failed to read {}: {}", st3_path, e))?;
        if !Self::is_star3(&data) {
            return Err("Not a STAR DATA V3.50 file".to_string());
        }

        // Criar estrutura básica de um MIDI Format 0 (1 Track)
        let mut mid = Vec::new();
        mid.extend_from_slice(b"MThd");
        mid.extend_from_slice(&[0, 0, 0, 6]);
        mid.extend_from_slice(&[0, 0]); // Format 0
        mid.extend_from_slice(&[0, 1]); // 1 Track
        mid.extend_from_slice(&[0x01, 0xE0]); // 480 PPQN (chute inicial para a resolução do delta)

        mid.extend_from_slice(b"MTrk");
        
        let mut track_data = Vec::new();

        // O cabeçalho principal tem 0x111 bytes.
        let mut idx = 0x111;
        
        // Pular a tabela de chunks
        if idx + 24 <= data.len() {
            idx += 24; // 6 inteiros
        }
        
        if idx + 4 <= data.len() {
            let chunk_len = u32::from_le_bytes([data[idx], data[idx+1], data[idx+2], data[idx+3]]) as usize;
            idx += 4;
            
            if idx + chunk_len <= data.len() {
                let chunk_data = &data[idx..idx+chunk_len];
                
                if chunk_data.starts_with(b"CEVENT") {
                    // CEVENT tem 6 bytes + 0 padding. Vamos procurar sequências que parecem Delta Time (4 bytes) + Status Byte MIDI (1 byte, >= 0x80)
                    let mut i = 12; // Pular "CEVENT" e padding inicial
                    while i + 5 <= chunk_data.len() {
                        let status_byte = chunk_data[i+4];
                        
                        // Heurística: É um status byte MIDI (Note On, Note Off, CC, Program Change)
                        if status_byte >= 0x80 && status_byte < 0xF0 {
                            let delta = u32::from_le_bytes([chunk_data[i], chunk_data[i+1], chunk_data[i+2], chunk_data[i+3]]);
                            
                            // Determinar tamanho do evento MIDI
                            let evt_len = match status_byte & 0xF0 {
                                0xC0 | 0xD0 => 2,
                                _ => 3,
                            };
                            
                            if i + 4 + evt_len <= chunk_data.len() {
                                // Escrever VLQ Delta
                                Self::write_vlq(delta, &mut track_data);
                                // Escrever Evento
                                track_data.extend_from_slice(&chunk_data[i+4 .. i+4+evt_len]);
                                
                                i += 4 + evt_len;
                                continue;
                            }
                        }
                        
                        // Avançar apenas 1 byte e procurar novamente se não bateu
                        i += 1;
                    }
                }
            }
        }
        
        // Escrever Fim da Track (Meta Event)
        track_data.extend_from_slice(&[0x00, 0xFF, 0x2F, 0x00]);

        // Escrever o tamanho da track no arquivo MIDI
        let track_len = track_data.len() as u32;
        mid.extend_from_slice(&track_len.to_be_bytes());
        mid.extend_from_slice(&track_data);

        // Salvar em um arquivo temporário
        let temp_dir = std::env::temp_dir();
        let mid_path = temp_dir.join(format!("{}.mid", uuid::Uuid::new_v4()));
        
        std::fs::write(&mid_path, mid).map_err(|e| format!("Failed to write temp MIDI: {}", e))?;
        
        Ok(mid_path.to_string_lossy().to_string())
    }

    fn write_vlq(mut val: u32, buf: &mut Vec<u8>) {
        let mut buffer = [0u8; 4];
        let mut i = 0;
        buffer[i] = (val & 0x7F) as u8;
        val >>= 7;
        while val > 0 {
            i += 1;
            buffer[i] = ((val & 0x7F) | 0x80) as u8;
            val >>= 7;
        }
        for j in (0..=i).rev() {
            buf.push(buffer[j]);
        }
    }
}
