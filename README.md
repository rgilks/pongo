# Pongo

A multiplayer Pong game built with **Rust + WebGPU** (client) and **Cloudflare Durable Objects** (server).

**[Play now →](https://pongo.rob-gilks.workers.dev)**

## Quick Start

```bash
cargo install wasm-pack       # Prerequisites: Rust, Node 20+
npx wrangler login            # One-time Cloudflare auth
npm run build && npm run dev  # http://localhost:8787
```

## How to Play

| Mode | How |
|------|-----|
| **Multiplayer** | CREATE → share code → JOIN |
| **VS AI** | Click VS AI |

**Controls:** Arrow keys or W/S · Touch on mobile  
**Rules:** First to 11. Hit position affects ball trajectory.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Cloudflare Workers                        │
│  ┌─────────────┐     ┌─────────────────────────────────┐    │
│  │lobby_worker │────▶│ server_do (Durable Object)      │    │
│  │  HTTP API   │     │  60Hz sim, WebSocket broadcast  │    │
│  └─────────────┘     └─────────────────────────────────┘    │
│                              │ uses                          │
│                              ▼                               │
│                      ┌───────────────┐                       │
│                      │  game_core    │◀─────────┐            │
│                      │  ECS + Physics│          │            │
│                      └───────────────┘          │            │
│                              ▲                  │            │
└──────────────────────────────│──────────────────│────────────┘
                               │ uses             │ uses
                       ┌───────┴───────┐   ┌──────┴──────┐
                       │    proto      │   │ client_wasm │
                       │ C2S/S2C msgs  │   │ WebGPU + UI │
                       └───────────────┘   └─────────────┘
                                                  │
                                                  ▼
                                              Browser
```

**Key design decisions:**
- **Shared `game_core`** — Same ECS physics on client and server for prediction
- **Binary protocol** — Minimal `postcard` serialization over WebSocket
- **Durable Objects** — Each match is a stateful instance with 60Hz game loop

## Project Structure

```
pongo/
├── game_core/       # ECS game logic (hecs) — shared by client/server
├── proto/           # Network protocol (postcard)
├── client_wasm/     # WebGPU renderer (wgpu)
├── server_do/       # Durable Object server
├── lobby_worker/    # HTTP endpoints + routing
└── worker/          # Built WASM + assets
```

## Commands

```bash
npm run build        # Build WASM
npm run dev          # Local server
npm run test         # Run tests  
npm run deploy       # Deploy to Cloudflare
```

## Troubleshooting

| Issue | Fix |
|-------|-----|
| Build fails | `cargo install wasm-pack` |
| Port in use | Kill process or edit `wrangler.toml` |
| Reset state | Delete `.wrangler/state/` |

See **[SPEC.md](SPEC.md)** for technical details and **[ARCHITECTURE.md](ARCHITECTURE.md)** for a deep dive into how it works.

## License

MIT
