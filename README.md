# ISO

A mobile-friendly, code-to-join, isometric arena shooter built with Rust + WebGPU (client) and Cloudflare Durable Objects (server).

## Current Status

**Milestone 2 (M2) - DO + Net: ✅ Complete**

- ✅ Cloudflare Workers infrastructure set up
- ✅ Durable Object created and deployed
- ✅ WebSocket support implemented
- ✅ Game simulation integrated into Durable Object
- ✅ Lobby endpoints (`/create`, `/join/:code`) working
- ✅ Network protocol implemented (C2S/S2C messages)
- ✅ Protocol parsing and snapshot generation
- ✅ Player joining logic
- ✅ Snapshot broadcasting

**Deployed at:** https://iso.rob-gilks.workers.dev

## Quick Start

### Prerequisites

- Rust (stable, 2021 edition)
- Node 20+
- wasm-pack (for M3)
- Wrangler CLI (`npm install -g wrangler`)

### Development

```bash
# Run all checks (fmt, clippy, tests)
npm run check:rust

# Format code
npm run fmt

# Run tests
npm run test

# Run clippy
npm run clippy

# Deploy to Cloudflare
npx wrangler deploy
```

### Pre-commit Hook

The project includes a pre-commit hook that automatically runs:
- `cargo fmt --check`
- `cargo clippy --workspace -- -D warnings`
- `cargo test --workspace`

The hook is configured via `git config core.hooksPath .githooks`.

## Project Structure

```
iso/
├── game_core/      # hecs ECS, systems, components, params
├── proto/          # C2S/S2C, quantization, versioning
├── client_wasm/    # wgpu renderer, input, prediction, WS
├── server_do/      # Durable Object Match: WS hub, step, storage, bots
├── lobby_worker/   # /create /join/:code, serves client
└── assets/         # eyeball textures, meshes, sfx
```

## Testing

See `TEST-PLAN.md` for detailed test procedures.

**Quick test:**
```bash
# Create a match
curl https://iso.rob-gilks.workers.dev/create

# Join a match (replace CODE with actual code)
curl https://iso.rob-gilks.workers.dev/join/CODE
```

**Unit tests:**
- `cargo test --package game_core` - Core game logic
- `cargo test --package proto` - Protocol serialization

## Documentation

- **Specification**: `SPEC.md`
- **Test Plan**: `TEST-PLAN.md`

## License

MIT
