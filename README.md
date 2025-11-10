# PONG

A multiplayer Pong game built with Rust + WebGPU (client) and Cloudflare Durable Objects (server).

This is a simplified version that demonstrates the architecture with the classic 1970s Pong game mechanics.

## Current Status

**Game Mechanics**

- Two paddles (one per player)
- Ball bounces between paddles and walls
- Score tracking
- Simple physics simulation

**Architecture**

- ✅ Rust WASM client with WebGPU rendering
- ✅ Cloudflare Durable Objects for server
- ✅ WebSocket-based networking
- ✅ ECS architecture (hecs)
- ✅ Client-server synchronization

**Deployed at:** https://iso.rob-gilks.workers.dev

## Game Rules

- Two players control paddles on opposite sides of the screen
- Ball bounces off walls and paddles
- Miss the ball and your opponent scores
- First to 11 points wins (classic Pong rules)
- Up/Down keys to move your paddle

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
3. Join match: Enter the code in a second browser window/tab
4. Play Pong: Use Up/Down arrow keys to move your paddle
5. Watch the ball bounce and score updates

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
pong/
├── game_core/      # hecs ECS, paddle/ball systems, components
├── proto/          # C2S/S2C messages
├── client_wasm/    # wgpu renderer, input, WS
├── server_do/      # Durable Object Match: WS hub, game loop
├── lobby_worker/   # /create /join/:code, serves client
└── assets/         # (minimal assets needed)
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
3. Join the match from a second browser window
4. Test paddle movement with Up/Down arrow keys
5. Watch the ball bounce and score updates

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
