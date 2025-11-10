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
- ✅ WGSL shader alignment fixed (uniform buffer 16-byte alignment)
- ✅ Periodic game loop (50ms ticks, 20 ticks/sec) via Durable Object alarms
- ✅ Snapshot broadcasting to all connected clients
- ⏳ Game entity rendering (players, bolts, blocks) - infrastructure ready
- ⏳ Mobile and desktop controls
- ⏳ Client prediction and reconciliation
- ⏳ HDR target and bloom post-processing

**Deployed at:** https://iso.rob-gilks.workers.dev

## Cost Optimization

**ISO is optimized for minimal Cloudflare costs** during development and production:

- ✅ **30 Hz client input rate** with coalescing (only send on change)
- ✅ **Automatic alarm shutdown** when no players connected
- ✅ **WebSocket error handling** prevents reconnection loops
- ✅ **Free tier capacity**: ~10-12 full matches/day (6 players, 10 min each)
- ✅ **Paid tier costs**: $5-7/month for 10-50 matches/day

**For detailed cost information, see:**

- [`COST-OPTIMIZATION-SUMMARY.md`](./COST-OPTIMIZATION-SUMMARY.md) - Quick overview
- [`COST-OPTIMIZATION.md`](./COST-OPTIMIZATION.md) - Complete guide

## Quick Start

### Prerequisites

- **Rust** (stable, 2021 edition)
- **Node 20+**
- **wasm-pack** (for client WASM builds): `cargo install wasm-pack`
- **Wrangler CLI**: `npm install -g wrangler` or use `npx wrangler`
- **Cloudflare account**: `npx wrangler login` (one-time setup)

### First Time Setup

```bash
# 1. Install dependencies (if needed)
cargo install wasm-pack

# 2. Login to Cloudflare (one-time)
npx wrangler login

# 3. Build the project
npm run build

# 4. Start local development
npm run dev
```

Visit `http://localhost:8787` to see the game!

### Development Workflow

**Standard cycle (see `DEVELOPMENT.md` for details):**

```bash
# 1. Make changes, then verify
npm run test:all     # Format, lint, test (required before commit)

# 2. Build and test locally
npm run build        # Build client + server WASM
npm run dev          # Start local dev server at http://localhost:8787

# 3. Deploy and verify
npm run deploy:test  # Deploy + test endpoints + check logs

# 4. Commit (pre-commit hook runs checks automatically)
git add -A && git commit -m "Description of changes"
git push
```

**Quick commands:**

```bash
npm run fmt          # Format code
npm run test         # Run tests
npm run clippy       # Run clippy
npm run logs         # View Cloudflare logs (real-time)
```

**See `DEVELOPMENT.md` for complete workflow details.**

### Local Development & Testing

**Prerequisites for local testing:**

- Build the project first: `npm run build`
- Ensure you're logged in: `npx wrangler login` (one-time setup)

**Start local development server:**

```bash
# Build client and server WASM
npm run build

# Start local dev server (uses Miniflare for Durable Objects)
npm run dev
# Server starts at http://localhost:8787
```

**Testing locally:**

1. Open browser to `http://localhost:8787`
2. Create a match: Visit `http://localhost:8787/create` to get a match code
3. Join match: Enter the code in the browser UI and click "Join Match"
4. Test WebSocket connections - they work locally via Miniflare
5. Test rendering - all entities should render correctly

**Verified working locally:**

- ✅ Match creation endpoint (`/create`)
- ✅ Match join endpoint (`/join/:code`)
- ✅ WebSocket connection establishment
- ✅ Client WASM initialization
- ✅ WebGPU rendering setup

**Benefits of local testing:**

- ✅ No rate limits - test as much as you want
- ✅ Faster iteration - no deployment needed
- ✅ Better debugging - see logs in terminal
- ✅ Isolated - doesn't affect production

**Remote development (optional):**

```bash
# Test against Cloudflare infrastructure but run locally
npx wrangler dev --remote --assets worker/pkg
```

**Troubleshooting local development:**

- If the server doesn't start, ensure you've run `npm run build` first
- If WebSocket connections fail, check the terminal for error messages
- Local state is persisted in `.wrangler/state/` - delete this folder to reset
- Make sure port 8787 is not already in use

### Deployment

```bash
# Deploy and test (deploy + endpoint tests + log checking)
npm run deploy:test

# Or deploy only
npx wrangler deploy  # Deploys to https://iso.<your-subdomain>.workers.dev

# Check Cloudflare logs
npm run logs         # Real-time tail
npm run logs:check   # Automated check (10 seconds)
```

### Pre-commit Hook

The project includes a pre-commit hook that automatically runs checks before each commit:

- `cargo fmt --check` - Format verification
- `cargo clippy --workspace -- -D warnings` - Linting
- `cargo test --workspace` - All tests

**Setup (one-time):**

```bash
git config core.hooksPath .githooks
```

The hook prevents commits if any check fails, ensuring code quality. See `DEVELOPMENT.md` for the complete workflow.

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

### Local Testing

**Setup:**

```bash
# 1. Build the project
npm run build

# 2. Start local dev server
npm run dev
# Server runs at http://localhost:8787
```

**Test in browser:**

1. Open `http://localhost:8787` in your browser
2. Create a match: Visit `http://localhost:8787/create` (or use the UI)
3. Join the match using the code shown
4. Test WebSocket connection and entity rendering
5. Test controls: W/S (move), A/D (turn), 1/2/3 (fire), Q/E/R (shield)

**Test endpoints locally:**

```bash
# Create a match (local)
curl http://localhost:8787/create

# Join a match (local, replace CODE with actual code)
curl http://localhost:8787/join/CODE
```

### Production Testing

**Test deployed endpoints:**

```bash
# Create a match
curl https://iso.rob-gilks.workers.dev/create

# Join a match (replace CODE with actual code)
curl https://iso.rob-gilks.workers.dev/join/CODE
```

### Automated Tests

**Run unit/integration tests:**

```bash
npm run test              # All tests
cargo test --package game_core  # Core game logic
cargo test --package proto      # Protocol serialization
```

## Documentation

- **Development Workflow**: `DEVELOPMENT.md` - Complete development cycle and best practices
- **Specification**: `SPEC.md` - Full game specification and architecture
- **Test Plan**: `TEST-PLAN.md` - Manual and automated test procedures
- **Completion Plan**: `COMPLETION-PLAN.md` - Detailed milestone tracking and next steps

## License

MIT
