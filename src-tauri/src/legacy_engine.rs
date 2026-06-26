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

    // =========================================================================
    // DECODER DE FORÇA-BRUTA: MK1 → MIDI  (v2 — varredura total de offset)
    // Estrutura MK1 confirmada: ~30 bytes de cabeçalho binário + nome do arquivo
    // (sem extensão) em ASCII + byte nulo + payload codificado.
    // Como o nome NÃO inclui ".kar", pesquisamos por offset via varredura XOR total.
    // =========================================================================

    /// Decodifica um arquivo .mk1 para MIDI temporário via força-bruta.
    pub fn decode_mk1_to_midi(path: &str) -> Result<String, String> {
        let data = fs::read(path).map_err(|e| format!("IO error ao ler MK1: {}", e))?;
        let mthd: &[u8] = b"MThd";
        let scan_limit = data.len().saturating_sub(4).min(4096);

        // 1. XOR estático — varredura completa de todos os offsets × todas as 256 chaves.
        //    Complexidade: O(scan_limit × 256) ≈ ~1M comparações → millisegundos.
        for offset in 0..=scan_limit {
            // 1.1 Teste 1-byte XOR
            for key in 0u8..=255 {
                if data[offset]   ^ key == mthd[0]
                && data[offset+1] ^ key == mthd[1]
                && data[offset+2] ^ key == mthd[2]
                && data[offset+3] ^ key == mthd[3] {
                    let decoded: Vec<u8> = data[offset..].iter().map(|&b| b ^ key).collect();
                    if Self::is_valid_midi(&decoded) {
                        println!("[MK1] XOR estático 1-byte: offset={} key=0x{:02X}", offset, key);
                        return Self::save_temp_midi(&decoded);
                    }
                }
            }
            
            // 1.2 Teste 4-byte cyclic XOR ( deduced directly from MThd )
            let mut k4 = [0u8; 4];
            for i in 0..4 { k4[i] = data[offset + i] ^ mthd[i]; }
            let mut test_buf4 = vec![0u8; 32.min(data.len() - offset)];
            for i in 0..test_buf4.len() { test_buf4[i] = data[offset + i] ^ k4[i % 4]; }
            
            if Self::is_valid_midi(&test_buf4) {
                let decoded: Vec<u8> = data[offset..].iter().enumerate().map(|(i, &b)| b ^ k4[i % 4]).collect();
                if Self::is_valid_midi(&decoded) { // verify full is actually needed
                    println!("[MK1] XOR cíclico 4-byte: offset={} key={:?}", offset, k4);
                    return Self::save_temp_midi(&decoded);
                }
            }
            
            // 1.3 Teste 2-byte cyclic XOR ( deduced from MT )
            let mut k2 = [0u8; 2];
            k2[0] = data[offset] ^ mthd[0];
            k2[1] = data[offset+1] ^ mthd[1];
            if data[offset+2] ^ k2[0] == mthd[2] && data[offset+3] ^ k2[1] == mthd[3] {
                let mut test_buf2 = vec![0u8; 32.min(data.len() - offset)];
                for i in 0..test_buf2.len() { test_buf2[i] = data[offset + i] ^ k2[i % 2]; }
                
                if Self::is_valid_midi(&test_buf2) {
                    let decoded: Vec<u8> = data[offset..].iter().enumerate().map(|(i, &b)| b ^ k2[i % 2]).collect();
                    if Self::is_valid_midi(&decoded) {
                        println!("[MK1] XOR cíclico 2-byte: offset={} key={:?}", offset, k2);
                        return Self::save_temp_midi(&decoded);
                    }
                }
            }
        }

        // 2. Subtração estática — mesma lógica, (byte - key) mod 256.
        for offset in 0..=scan_limit {
            for key in 1u8..=255 {
                if data[offset]  .wrapping_sub(key) == mthd[0]
                && data[offset+1].wrapping_sub(key) == mthd[1]
                && data[offset+2].wrapping_sub(key) == mthd[2]
                && data[offset+3].wrapping_sub(key) == mthd[3] {
                    let decoded: Vec<u8> = data[offset..].iter().map(|&b| b.wrapping_sub(key)).collect();
                    if Self::is_valid_midi(&decoded) {
                        println!("[MK1] Subtração: offset={} key=0x{:02X}", offset, key);
                        return Self::save_temp_midi(&decoded);
                    }
                }
            }
        }
        let data = fs::read(path).map_err(|e| format!("IO error: {}", e))?;
        if data.len() < 30 || data[0..4] != [0x87, 0x0a, 0xd6, 0x30] {
            return Err("Not a valid MK1 file (invalid header)".to_string());
        }

        let name_len = (data[26] as usize) | ((data[27] as usize) << 8);
        let payload_start = 30 + name_len;
        
        let comp_size = (data[18] as usize) 
                      | ((data[19] as usize) << 8)
                      | ((data[20] as usize) << 16)
                      | ((data[21] as usize) << 24);
                      
        if payload_start + comp_size > data.len() {
            return Err("MK1 file truncated or corrupted".to_string());
        }
        
        let payload = &data[payload_start..payload_start + comp_size];
        
        struct XorReader<'a> {
            data: &'a [u8],
            key: &'a [u8],
            pos: usize,
        }
        impl<'a> std::io::Read for XorReader<'a> {
            fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
                let rem = self.data.len() - self.pos;
                if rem == 0 { return Ok(0); }
                let to_read = std::cmp::min(rem, buf.len());
                for i in 0..to_read {
                    buf[i] = self.data[self.pos + i] ^ self.key[(self.pos + i) % self.key.len()];
                }
                self.pos += to_read;
                Ok(to_read)
            }
        }

        // Test 1-byte keys
        for k in 0..=255u8 {
            let key = [k];
            let reader = XorReader { data: payload, key: &key, pos: 0 };
            let mut decoder = flate2::read::DeflateDecoder::new(reader);
            let mut uncompressed = Vec::new();
            if std::io::Read::read_to_end(&mut decoder, &mut uncompressed).is_ok() {
                if Self::is_valid_midi(&uncompressed) {
                    println!("[MK1] Decoded with 1-byte XOR key: 0x{:02X}", k);
                    return Self::save_temp_midi(&uncompressed);
                }
            }
        }

        // Test 2-byte keys
        for k0 in 0..=255u8 {
            for k1 in 0..=255u8 {
                let key = [k0, k1];
                let reader = XorReader { data: payload, key: &key, pos: 0 };
                let mut decoder = flate2::read::DeflateDecoder::new(reader);
                let mut uncompressed = Vec::new();
                if std::io::Read::read_to_end(&mut decoder, &mut uncompressed).is_ok() {
                    if Self::is_valid_midi(&uncompressed) {
                        println!("[MK1] Decoded with 2-byte XOR key: 0x{:02X}{:02X}", k0, k1);
                        return Self::save_temp_midi(&uncompressed);
                    }
                }
            }
        }

        Err("Failed to decrypt MK1 with 1-byte and 2-byte XOR".to_string())
    }

    /// Critério forte de MIDI válido:
    /// - Começa com MThd
    /// - Bytes 4-7 = 0x00 0x00 0x00 0x06 (tamanho do header MIDI = 6)
    /// - Bytes 14-17 = MTrk (primeiro track logo após os 14 bytes de header)
    fn is_valid_midi(data: &[u8]) -> bool {
        data.len() >= 22
            && &data[0..4] == b"MThd"
            && data[4] == 0 && data[5] == 0 && data[6] == 0 && data[7] == 6
            && data.len() > 18
            && &data[14..18] == b"MTrk"
    }

    /// Localiza o offset após o fim da primeira string ASCII longa no cabeçalho MK1.
    /// O header MK1 tem ~30 bytes binários seguidos pelo nome do arquivo (sem extensão).
    fn find_ascii_string_end(data: &[u8]) -> Option<usize> {
        let mut in_run = false;
        let mut run_start = 0usize;

        for i in 4..data.len().min(256) {
            let printable = data[i] >= 0x20 && data[i] < 0x7F;
            if printable && !in_run {
                in_run = true;
                run_start = i;
            } else if !printable && in_run {
                in_run = false;
                if i - run_start >= 5 {
                    // Sequência longa o suficiente para ser um nome de arquivo
                    let mut j = i;
                    while j < data.len().min(i + 64) && data[j] < 0x20 { j += 1; }
                    if j < data.len() {
                        println!("[MK1] Fim do nome do arquivo no offset {}, payload estimado no offset {}", i, j);
                        return Some(j);
                    }
                }
            }
        }
        None
    }

    /// Processa um arquivo .kara: pode ser MIDI puro ou MK1-encapsulado.
    pub fn decode_kara_to_midi(path: &str) -> Result<String, String> {
        let data = fs::read(path).map_err(|e| format!("IO error ao ler KARA: {}", e))?;

        if data.starts_with(b"MThd") {
            println!("[KARA] Arquivo MIDI puro detectado");
            return Self::save_temp_midi(&data);
        }

        // Tentar XOR estático em todo o arquivo (caso seja MK1-encapsulado)
        let mthd: &[u8] = b"MThd";
        let scan_limit = data.len().saturating_sub(4).min(4096);
        for offset in 0..=scan_limit {
            for key in 0u8..=255 {
                if data[offset]   ^ key == mthd[0]
                && data[offset+1] ^ key == mthd[1]
                && data[offset+2] ^ key == mthd[2]
                && data[offset+3] ^ key == mthd[3] {
                    let decoded: Vec<u8> = data[offset..].iter().map(|&b| b ^ key).collect();
                    if Self::is_valid_midi(&decoded) {
                        println!("[KARA] XOR estático 1-byte: offset={} key=0x{:02X}", offset, key);
                        return Self::save_temp_midi(&decoded);
                    }
                }
            }
            
            // 4-byte cyclic
            let mut k4 = [0u8; 4];
            for i in 0..4 { k4[i] = data[offset + i] ^ mthd[i]; }
            let mut test_buf4 = vec![0u8; 32.min(data.len() - offset)];
            for i in 0..test_buf4.len() { test_buf4[i] = data[offset + i] ^ k4[i % 4]; }
            if Self::is_valid_midi(&test_buf4) {
                let decoded: Vec<u8> = data[offset..].iter().enumerate().map(|(i, &b)| b ^ k4[i % 4]).collect();
                if Self::is_valid_midi(&decoded) {
                    println!("[KARA] XOR cíclico 4-byte: offset={} key={:?}", offset, k4);
                    return Self::save_temp_midi(&decoded);
                }
            }
            
            // 2-byte cyclic
            let mut k2 = [0u8; 2];
            k2[0] = data[offset] ^ mthd[0];
            k2[1] = data[offset+1] ^ mthd[1];
            if data[offset+2] ^ k2[0] == mthd[2] && data[offset+3] ^ k2[1] == mthd[3] {
                let mut test_buf2 = vec![0u8; 32.min(data.len() - offset)];
                for i in 0..test_buf2.len() { test_buf2[i] = data[offset + i] ^ k2[i % 2]; }
                if Self::is_valid_midi(&test_buf2) {
                    let decoded: Vec<u8> = data[offset..].iter().enumerate().map(|(i, &b)| b ^ k2[i % 2]).collect();
                    if Self::is_valid_midi(&decoded) {
                        println!("[KARA] XOR cíclico 2-byte: offset={} key={:?}", offset, k2);
                        return Self::save_temp_midi(&decoded);
                    }
                }
            }
        }

        Err("KARA: não é MIDI padrão nem decodificável por XOR simples".to_string())
    }

    /// Salva bytes em arquivo .mid temporário e retorna o caminho.
    fn save_temp_midi(data: &[u8]) -> Result<String, String> {
        let temp_dir = std::env::temp_dir();
        let filename = format!("ukp_legacy_{}.mid", uuid::Uuid::new_v4());
        let mid_path = temp_dir.join(&filename);
        std::fs::write(&mid_path, data)
            .map_err(|e| format!("Erro ao salvar MIDI temporário: {}", e))?;
        println!("[LEGACY] MIDI temporário salvo em: {}", mid_path.display());
        Ok(mid_path.to_string_lossy().to_string())
    }
}
