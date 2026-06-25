# 🎤 Ultimate Karaoke Player

Bem-vindo ao **Ultimate Karaoke Player**, a solução definitiva em software de karaokê! Diferente dos players tradicionais focados em apenas um tipo de arquivo, este projeto inovador traz uma arquitetura multi-engine (vários motores), garantindo que você possa reproduzir praticamente **qualquer formato** de karaokê existente na história.

Construído com uma base sólida em **Rust** (para máxima performance e manipulação a nível de byte) e uma interface moderna em **React + Tauri**, ele não só reproduz as músicas como transforma a experiência em um verdadeiro evento.

---

## 🚀 Arquitetura Multi-Engine

O Ultimate Karaoke Player foi desenhado com módulos separados para cada família de formatos. Isso permite que ele entenda e execute as nuances exatas de cada tipo de arquivo.

### 1. CDG Engine (O Clássico)
**Formatos suportados:** `.cdg` + `.mp3` / `.ogg` / `.wav`
- Lê diretamente os pacotes de instrução de desenho (Draw Instructions, XOR Colors, Memory Presets) do formato nativo CD-Graphics.
- Renderização perfeita a ~60FPS direto na tela com sincronização de áudio impecável.

### 2. Synth Engine (Síntese MIDI)
**Formatos suportados:** `.kar`, `.mid`
- Extrai canais de texto (Lyrics/Text Events) contidos dentro dos arquivos MIDI.
- Sincroniza a letra com as batidas de tempo internas (PPQN - Pulses Per Quarter Note).
- Renderiza um visual moderno e limpo, atualizando a sílaba cantada milissegundo a milissegundo.

### 3. Video Engine (Clipes com Legendas)
**Formatos suportados:** `.mp4`, `.mkv`, `.avi` + `.lrc`, `.srt`, `.ass`
- Aproveita a aceleração de hardware nativa para rodar vídeos pesados.
- Sobrepõe legendas através de parsers robustos, garantindo a sincronia da letra com o vídeo em reprodução de fundo.

### 4. Ultrastar Engine (A Experiência Gamificada)
**Formatos suportados:** `.txt` (Ultrastar Deluxe) + Áudio + Vídeo/Background
- **Motor Gamificado 100% Funcional!** 
- Exibe o famoso **Piano Roll 3D**, guiando o cantor nota por nota.
- **Detecção de Voz Real-Time:** Utiliza auto-correlação matemática para escutar sua voz pelo microfone, descobrir a frequência cantada e normalizar as oitavas na tela.
- **Sistema de Pontuação:** Score dinâmico até 10.000 pontos com as clássicas classificações ao final da música (*Amateur, Rising Star, Superstar, Ultrastar!*).
- **Mídia Dinâmica:** Suporta vídeos, capas de álbuns e contagens regressivas imersivas (Lead-In).

### 5. Tracker Engine (Old School)
**Formatos suportados:** `.mod`, `.s3m`, `.xm`, `.it`
- Parse de cabeçalhos de arquivos Tracker clássicos da era Amiga/MS-DOS. 

### 6. Legacy Engine (Máquinas Antigas)
**Formatos suportados:** `.mk1`, `.kara`
- Desvenda os metadados de arquivos binários proprietários usados em máquinas de karaokê comerciais das décadas passadas.

---

## ⚙️ Tecnologias Utilizadas

- **Rust:** Todo o peso do parsing, scanner recursivo de diretórios (que lida com milhares de arquivos instantaneamente) e bancos de dados SQLite (com a biblioteca `sqlx`).
- **Tauri:** Uma alternativa super leve ao Electron. Provê os binários do OS (Windows, Linux, MacOS) com uma WebView super veloz.
- **React 18 + TailwindCSS:** Para uma interface de usuário extremamente bonita, responsiva e com efeitos visuais fluidos.
- **Rodio:** Biblioteca pura em Rust para manipulação, controle de pitch e playback de áudio.

---

## 📦 Como Instalar e Rodar Localmente

Certifique-se de ter instalado na sua máquina:
- [Node.js](https://nodejs.org/en/) (Versão 18+)
- [Rust](https://www.rust-lang.org/tools/install)
- [Dependências de Build do Tauri](https://tauri.app/v1/guides/getting-started/prerequisites)

1. Clone este repositório:
   ```bash
   git clone https://github.com/walterfr/UltimateKaraokePlayer.git
   cd UltimateKaraokePlayer
   ```

2. Instale as dependências Node:
   ```bash
   npm install
   ```

3. Inicie o ambiente de desenvolvimento (React App + Rust Backend simultaneamente):
   ```bash
   npm run tauri dev
   ```

## 🛠 Fazendo o Build (Release)

Para gerar um instalador otimizado (`.exe`, `.msi`, `.deb`, `.app`) para distribuir:
```bash
npm run tauri build
```
O executável final estará disponível na pasta `src-tauri/target/release/bundle/`.

---

## 📝 Próximos Passos (Roadmap)
- [ ] Concluir o **Client Mode**: Interface web para smartphones na mesma rede local, permitindo que a plateia adicione músicas à fila remotamente.
- [ ] Comunicação em Tempo Real: Adicionar WebSockets.
- [ ] Reprodução em tempo real dos samples no Tracker Engine.
- [ ] Finalizar integração de reprodução sintetizada via SoundFonts (FluidSynth) para os MIDIs.

---

### Licença
Este projeto é provido sob a licença MIT. Divirta-se cantando!
