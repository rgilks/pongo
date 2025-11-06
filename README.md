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

**Milestone 3 (M3) - Client WebGPU: 🚧 In Progress**

- ✅ Client WASM crate structure created
- ✅ WebGPU surface initialization (wgpu 24.0 with webgpu feature)
- ✅ Isometric camera with view/projection matrices
- ✅ Basic rendering pipeline (meshes, shader, forward pass, Lambert lighting)
- ✅ Light buffers (SSBO for up to 8 point lights)
- ⏳ Game entity rendering (players, bolts, blocks)
- ⏳ Mobile and desktop controls
- ⏳ Client prediction and reconciliation
- ⏳ HDR target and bloom post-processing

**Deployed at:** https://iso.rob-gilks.workers.dev

## Quick Start

### Prerequisites

- Rust (stable, 2021 edition)
- Node 20+
- wasm-pack (for client WASM builds)
- Wrangler CLI: `npm install -g wrangler`
- Cloudflare account: `wrangler login`

### Development

```bash
# Run all checks (fmt, clippy, tests) - recommended
npm run test:all

# Individual commands
npm run fmt          # Format code
npm run test         # Run tests
npm run clippy       # Run clippy

# Local development server
wrangler dev         # Starts at http://localhost:8787

# Deploy and test (deploy + endpoint tests + log checking)
npm run deploy:test

# Or deploy only
npx wrangler deploy  # Deploys to https://iso.<your-subdomain>.workers.dev

# Check Cloudflare logs
npm run logs         # Real-time tail
npm run logs:check   # Automated check (10 seconds)
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

**Quick endpoint test:**
```bash
# Create a match
curl https://iso.rob-gilks.workers.dev/create

# Join a match (replace CODE with actual code)
curl https://iso.rob-gilks.workers.dev/join/CODE
```

**Run tests:**
```bash
npm run test              # All tests
cargo test --package game_core  # Core game logic
cargo test --package proto      # Protocol serialization
```

## Documentation

- **Specification**: `SPEC.md` - Full game specification and architecture
- **Test Plan**: `TEST-PLAN.md` - Manual and automated test procedures

## License

MIT
