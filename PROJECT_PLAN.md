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
1. [x] CDG Engine (MP3+G) - *Parser, Áudio e Sincronização concluídos!*
2. [x] Synth Engine (MIDI/KAR) - *Parser MIDI, extração de letras e renderização concluídos!*
3. [x] Video Engine (MP4/ASS/LRC/SRT) - *Player nativo + overlay de legendas concluído!*
4. [x] Ultrastar Engine (TXT+MP3) - *Parser de notas estilo SingStar*
5. [x] Tracker Engine (MOD/XM/S3M) - *Parser de cabeçalhos e estrutura concluído!*
6. [x] Legacy Engine (MK1/KARA) - *Parser de cabeçalhos MK1/KARA concluído!*

✅ **Fase 1 completa - todos os 6 motores implementados!**

### Fase 2: Core do Servidor e Gestão
- [x] Implementação do banco de dados SQLite (schema songs/queue/settings)
- [x] Sistema de indexação de arquivos (scanner recursivo com auto-detecção de formato)
- [x] API de gerenciamento de fila (enqueue, remove, reorder, clear) e configurações
- [x] Comandos Tauri para busca de músicas, ordenação (sort) e fila
- [x] Interface de biblioteca (busca inteligente + tags + clear/sort) conectada ao BD
- [x] Interface de fila (enqueue/remove com auto-play e auto-refresh)
- [ ] Servidor HTTP Axum + WebSocket ← **PRÓXIMO**

### Fase 3: Interface e Sincronização
- [x] Desenvolvimento do Player Host no Tauri (Painel de Configuração de Motores, Queue e Biblioteca centralizada).
- [ ] Desenvolvimento da Interface de Cliente Web Remoto (Browser de Celular).
- [ ] Sincronização de estado via WebSockets.

### Fase 4: Refinamento e Testes
- [ ] Otimização de latência.
- [ ] Testes de estresse com múltiplos clientes.
- [ ] Polimento de UI/UX.

## 📅 Próximos Passos (Sessão Atual)
- [x] Implementar servidor HTTP Axum para API REST de biblioteca/fila
- [x] Adicionar suporte WebSocket para sincronização em tempo real
- [x] Criar interface web para clientes remotos
- [ ] Finalizar integração de áudio MIDI com fluid-synth
