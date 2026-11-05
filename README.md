# Relíquias Launcher

Launcher do servidor Relíquias SMP (Minecraft 1.21.1, NeoForge). Instala Java, Minecraft, NeoForge e o pacote de mods, entra com a conta do Discord e mantém tudo atualizado a cada abertura.

## Estrutura

- `core/`: biblioteca Rust e CLI `rl` (login, sincronia do pacote, instalação, lançamento do jogo).
- `app/`: aplicativo desktop em Tauri 2, com interface em Svelte 5 e Vite.
- `vendor/`: dependência `self-replace` usada na autoatualização.

## Build

Em `launcher/` (a pasta raiz deste repositório):

```powershell
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo build --release -p launcher-core
```

CLI:

```powershell
cargo run -p launcher-core -- login
cargo run -p launcher-core -- sincronizar
cargo run -p launcher-core -- lancar --dry-run
cargo run -p launcher-core -- verificar
```

Aplicativo:

```powershell
cd app
npm ci
npm run tauri build
```

Requisitos: Rust 1.88 ou mais novo, Node.js e WebView2 (Windows 10/11).

## Configuração do build

Valores públicos, embutidos na compilação (não contêm segredos):

- `RL_SERVIDOR`: `host:porta` da API.
- `RL_SPKI`: SHA-256 do SPKI do certificado do servidor, em 64 caracteres hexadecimais.
- `RL_DISCORD_CLIENT_ID`: ID público da aplicação Discord. Sem ele o login falha antes de abrir o navegador.
- `RL_UPDATE_PUBKEY`: chave pública Ed25519 de atualização, em base64. Sem ela a autoatualização fica indisponível. A assinatura cobre o executável inteiro, e SHA-256 e tamanho também são verificados.

## Segurança

Os tokens ficam apenas no Windows Credential Manager, sem arquivo de sessão. Downloads são verificados por SHA-256 e retomados após queda de conexão. A conexão com a API usa pinning do certificado.
