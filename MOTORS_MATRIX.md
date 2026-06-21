# Matriz de Motores - Ultimate Karaoke Player

Esta matriz define os motores de renderização e reprodução necessários para garantir a compatibilidade total de formatos de karaokê.

## 1. Synth Engine (Síntese MIDI)
*   **Formatos:** `.kar`, `.mid`, `.smi`
*   **Tecnologia:** `fluid-synth` + SoundFonts (.sf2)
*   **Objetivo:** Transformar dados de eventos MIDI em áudio sintetizado com letras sincronizadas.

## 2. CDG Engine (Gráficos CDG)
*   **Formatos:** `.cdg` + (`.mp3`, `.wav`, `.ogg`, `.flac`)
*   **Tecnologia:** Parser de bytes CDG + `rodio` (Áudio) + Canvas Direct Pixel Manipulation (Gráficos) + Tauri IPC Master Clock
*   **Objetivo:** Renderizar os pacotes de desenho do padrão CD Graphics em sincronia com o arquivo de áudio.
*   **Status:** ✅ **Concluído e Integrado** (Lê bytes, gera cores XOR, renderiza ImageData via IPC React a ~60FPS).

## 3. Video Engine (Vídeo e Legendas)
*   **Formatos:** `.mp4`, `.mkv`, `.avi` + (`.lrc`, `.ass`, `.ssa`)
*   **Tecnologia:** `ffmpeg-next` / `GStreamer`
*   **Objetivo:** Decodificação de vídeo de alta performance e renderização de legendas avançadas.

## 4. Tracker Engine (Sample-based)
*   **Formatos:** `.st3`, `.s3m`, `.xm`, `.mod`
*   **Tecnologia:** Implementação de síntese baseada em amostras (Samples)
*   **Objetivo:** Reproduzir músicas no formato de trackers clássicos.

## 5. Legacy Engine (Formatos Antigos)
*   **Formatos:** `.mk1`, `.kara`
*   **Tecnologia:** Reverse engineering de headers binários
*   **Objetivo:** Suporte a formatos proprietários de máquinas de karaokê antigas.
