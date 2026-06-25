# Matriz de Motores - Ultimate Karaoke Player

Esta matriz define os motores de renderização e reprodução necessários para garantir a compatibilidade total de formatos de karaokê.

## 1. Synth Engine (Síntese MIDI)
*   **Formatos:** `.kar`, `.mid`, `.smi`
*   **Tecnologia:** `midly` (Parser MIDI) + `fluid-synth` SoundFonts (.sf2) + `rodio` (Áudio)
*   **Objetivo:** Transformar dados de eventos MIDI em áudio sintetizado com letras sincronizadas.
*   **Status:** ✅ **Concluído e Integrado** (Parser MIDI extrai letras KAR, metadados, tempo, eventos de nota; renderização de letras em tempo real com scroll automático).

## 2. CDG Engine (Gráficos CDG)
*   **Formatos:** `.cdg` + (`.mp3`, `.wav`, `.ogg`, `.flac`)
*   **Tecnologia:** Parser de bytes CDG + `rodio` (Áudio) + Canvas Direct Pixel Manipulation (Gráficos) + Tauri IPC Master Clock
*   **Objetivo:** Renderizar os pacotes de desenho do padrão CD Graphics em sincronia com o arquivo de áudio.
*   **Status:** ✅ **Concluído e Integrado** (Lê bytes, gera cores XOR, renderiza ImageData via IPC React a ~60FPS).

## 3. Video Engine (Vídeo e Legendas)
*   **Formatos:** `.mp4`, `.mkv`, `.avi` + (`.lrc`, `.ass`, `.ssa`, `.srt`)
*   **Tecnologia:** `<video>` HTML5 (HW accel) + `regex` (Parsing de legendas) + `convertFileSrc` (Asset Protocol)
*   **Objetivo:** Reprodução de vídeo com aceleração de hardware, overlay de legendas com suporte a LRC, SRT e ASS/SSA.
*   **Status:** ✅ **Concluído e Integrado** (Parser de legendas LRC/SRT/ASS, renderização de vídeo nativo com overlay de texto sincronizado).

## 4. Ultrastar Engine (Pitch-based Game)
*   **Formatos:** `.txt` (Ultrastar Deluxe header + notas) + `.mp3`/`.ogg` (áudio) + `.jpg`/`.png`/`.mp4` (background)
*   **Tecnologia:** Parser de metadados + Auto-Correlação Matemática (Pitch Detection via Web Audio API) + CSS 3D Rendering
*   **Objetivo:** Interpretar o formato de karaokê do Ultrastar Deluxe (SingStar-like) com mecânicas de jogo completas.
*   **Status:** ✅ **Concluído e Integrado** (Mecânica de jogo idêntica ao USDX: piano roll 3D, detecção de tom pelo microfone em tempo real, normalização de oitava, sistema de "fill" de nota, pontuação máxima de 10.000 pontos, tela de classificação de níveis e suporte a vídeos/imagens de fundo simultâneos).

## 5. Tracker Engine (Sample-based)
*   **Formatos:** `.mod`, `.s3m`, `.xm`, `.st3`, `.it`
*   **Tecnologia:** Parsers de cabeçalho MOD/S3M/XM (nativos em Rust) + `hound` (WAV export)
*   **Objetivo:** Extrair metadados de módulos tracker (título, instrumentos, canais, padrões) e reproduzir áudio sample-based.
*   **Status:** ✅ **Concluído e Integrado** (Parser de cabeçalhos MOD/S3M/XM, extração de metadados, exibição de estrutura na interface).

## 6. Legacy Engine (Formatos Antigos)
*   **Formatos:** `.mk1`, `.kara`
*   **Tecnologia:** Parsers de cabeçalho MK1 e KARA (nativos em Rust) + detecção automática de subformato (MIDI-based, CDG-based, raw_audio, text_based)
*   **Objetivo:** Analisar e extrair metadados de formatos proprietários de máquinas de karaokê antigas, detectando automaticamente o subformato para possível conversão.
*   **Status:** ✅ **Concluído e Integrado** (Parser de cabeçalhos MK1/KARA, detecção de subformato, extração de título/artista, dump hex do cabeçalho).
