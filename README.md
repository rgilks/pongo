# Pongo

A multiplayer Pong game built with **Rust + WebGPU** (client) and **Cloudflare Durable Objects** (server).

**[Play now →](https://pongo.rob-gilks.workers.dev)**

## Quick Start

```bash
# Prerequisites: Rust, Node 20+, wasm-pack
cargo install wasm-pack
npx wrangler login      # One-time Cloudflare auth

# Build and run locally
npm run build
npm run dev             # http://localhost:8787
```

## How to Play

| Mode | How |
|------|-----|
| **Multiplayer** | Click CREATE → share 5-char code → friend clicks JOIN |
| **VS AI** | Click VS AI for single-player |

**Controls:** Arrow keys or W/S (desktop) · Touch buttons (mobile)

**Rules:** First to 11 points wins. Ball trajectory affected by where and how you hit it.

## Project Structure

```
pongo/
├── game_core/       # ECS game logic (hecs)
├── proto/           # Network protocol (postcard)
├── client_wasm/     # WebGPU renderer (wgpu)
├── server_do/       # Durable Object server
├── lobby_worker/    # HTTP endpoints
└── worker/          # Built WASM + assets
```

## Commands

| Command | Description |
|---------|-------------|
| `npm run build` | Build WASM packages |
| `npm run dev` | Local server (localhost:8787) |
| `npm run test` | Run all tests |
| `npm run deploy` | Deploy to Cloudflare |
| `npm run logs` | View production logs |

**Full dev cycle:**
```bash
npm run test:all      # Format, lint, test
npm run build && npm run dev
npm run deploy:test   # Deploy + verify
```

## Troubleshooting

| Issue | Fix |
|-------|-----|
| Build fails | `cargo install wasm-pack` |
| Port 8787 in use | Kill process or edit `wrangler.toml` |
| Reset state | Delete `.wrangler/state/` |
| Not logged in | `npx wrangler login` |

## Documentation

- **[SPEC.md](SPEC.md)** — Technical specification
- **[TEST-PLAN.md](TEST-PLAN.md)** — Test procedures

## License

MIT
