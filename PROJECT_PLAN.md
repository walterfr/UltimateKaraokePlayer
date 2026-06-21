# Plano de Projeto - Ultimate Karaoke Player

## 🎯 Objetivo
Criar o player de karaokê definitivo com compatibilidade total de formatos, utilizando uma arquitetura híbrida de servidor/cliente.

## 🛠 Stack Tecnológica
- **Backend/Core:** Rust (Alta performance, segurança e concorrência)
- **Interface Host:** Tauri (WebView nativo + Rust)
- **Interface Cliente:** Web (React/Vue/Svelte + Tailwind CSS)
- **Comunicação:** WebSockets + HTTP API (Axum)
- **Banco de Dados:** SQLite (via SQLx)

## 🏗 Arquitetura
- **Híbrida:** O servidor roda dentro do Tauri.
- **Host Mode:** O servidor atua como player principal (áudio e vídeo locais).
- **Client Mode:** Usuários remotos acessam via browser para buscar e solicitar músicas.

## 🚀 Processo de Desenvolvimento

### Fase 1: Motores de Mídia (Atual)
Implementação individual de cada motor de reprodução, seguida de validação com arquivos de teste.
1. [ ] CDG Engine (MP3+G) - *Parser concluído, pendente renderização/áudio*
2. [ ] Synth Engine (MIDI/KAR)
3. [ ] Video Engine (MP4/ASS)
4. [ ] Tracker Engine (ST3/XM)
5. [ ] Legacy Engine (MK1)

### Fase 2: Core do Servidor e Gestão
- Implementação do banco de dados de músicas.
- Sistema de indexação de arquivos.
- API de gerenciamento de fila.

### Fase 3: Interface e Sincronização
- Desenvolvimento do Player Host no Tauri.
- Desenvolvimento da Interface de Cliente Web.
- Sincronização de tempo via WebSockets.

### Fase 4: Refinamento e Testes
- Otimização de latência.
- Testes de estresse com múltiplos clientes.
- Polimento de UI/UX.

## 📅 Próximos Passos (Sessão Atual)
- [x] Iniciar a implementação do primeiro motor (Sugerido: **CDG Engine** por ser a base do karaokê).
- [x] Configurar a estrutura básica do projeto Tauri/Rust.
- [x] Criar testes unitários para a decodificação de bytes do CDG.
- [ ] Criar a renderização visual do formato CDG na interface Web (via WebGL ou Canvas).
- [ ] Integrar a reprodução de áudio associada (`rodio`) e criar o laço de sincronização de tempo para as instruções CDG.
