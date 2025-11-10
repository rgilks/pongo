# PONG

A multiplayer Pong game built with Rust + WebGPU (client) and Cloudflare Durable Objects (server).

**Play now**: https://iso.rob-gilks.workers.dev

## What is This?

This is a modern implementation of the classic 1970s Pong game, demonstrating:

- **Rust + WebGPU** for client-side rendering (no game engine)
- **Cloudflare Durable Objects** for authoritative server
- **ECS architecture** (hecs) for game simulation
- **WebSocket** networking with client-server synchronization

## Game Rules

- Two players control paddles on opposite sides of the screen
- Ball bounces off walls and paddles
- Miss the ball and your opponent scores
- First to 11 points wins
- **Controls**: Up/Down arrow keys or W/S keys

## Quick Start

### Prerequisites

- **Rust** (stable, 2021 edition)
- **Node 20+**
- **wasm-pack**: `cargo install wasm-pack`
- **Cloudflare account**: `npx wrangler login` (one-time)

### Setup & Run Locally

```bash
# 1. Login to Cloudflare (one-time)
npx wrangler login

# 2. Build the project
npm run build

# 3. Start local dev server
npm run dev
```

Visit `http://localhost:8787` to play!

### Local Testing

1. Open browser to `http://localhost:8787`
2. Create a match (or visit `/create` endpoint to get a code)
3. Join from a second browser window
4. Use Up/Down arrow keys to control your paddle
5. First to 11 points wins!

**Benefits of local testing:**

- No rate limits
- Faster iteration
- Better debugging (see logs in terminal)
- Isolated from production

## Development Workflow

### Standard Cycle

```bash
# 1. Make changes, then verify
npm run test:all     # Format, lint, test

# 2. Build and test locally
npm run build        # Build client + server WASM
npm run dev          # Start local dev server at http://localhost:8787

# 3. Deploy and verify
npm run deploy:test  # Deploy + test endpoints + check logs

# 4. Commit and push
git add -A && git commit -m "Description of changes"
git push
```

### Individual Commands

```bash
npm run fmt          # Format code
npm run test         # Run tests
npm run clippy       # Run clippy linting
npm run build        # Build WASM packages
npm run dev          # Local dev server
npm run logs         # View Cloudflare logs (real-time)
npm run deploy       # Deploy to Cloudflare Workers
```

### Pre-commit Hook (Optional)

Automatically run checks before each commit:

```bash
# One-time setup
git config core.hooksPath .githooks
```

The hook runs:

- `cargo fmt --check` - Format verification
- `cargo clippy --workspace -- -D warnings` - Linting
- `cargo test --workspace` - All tests

## Project Structure

```
pong/
├── game_core/      # ECS (hecs): components, systems, game logic
├── proto/          # C2S/S2C network messages (postcard)
├── client_wasm/    # WebGPU renderer, input, WebSocket client
├── server_do/      # Durable Object: game loop, WebSocket hub
├── lobby_worker/   # HTTP endpoints (/create, /join/:code)
└── worker/         # Built WASM packages + static assets
```

## Architecture

### Client (Rust → WASM)

- **Rendering**: WebGPU via wgpu (v24.0)
- **Graphics**: 2D orthographic camera, simple colored shapes
- **Input**: Keyboard events (Up/Down or W/S)
- **Network**: WebSocket client, receives game state snapshots

### Server (Cloudflare Durable Objects)

- **Simulation**: Authoritative ECS (hecs) at 60 Hz
- **Networking**: WebSocket hub, broadcasts game state at 60 Hz
- **Match Lifecycle**: One Durable Object per match code
- **State**: Ball physics, paddle positions, score tracking

### Game Logic (game_core)

**Components:**

- `Paddle { player_id, y }` - Left/right paddles
- `Ball { pos, vel }` - Ball position and velocity
- `PaddleIntent { dir }` - Player input (-1 up, 0 stop, 1 down)

**Systems (deterministic order):**

1. **IngestInputs** - Apply player paddle commands
2. **MovePaddles** - Update paddle positions (clamped to arena)
3. **MoveBall** - Update ball position
4. **CheckCollisions** - Ball vs paddles, ball vs walls
5. **CheckScoring** - Detect when ball exits left/right edge
6. **ResetBall** - Reset after scoring

**Resources:**

- `Time { dt, now }` - Fixed timestep (16.67ms)
- `GameMap { width, height }` - Arena dimensions (32 × 24)
- `Score { left, right }` - Current scores
- `Config` - Game tuning (speeds, sizes, win condition)

### Network Protocol

**Client → Server (C2S):**

```rust
enum C2S {
    Join { code: [u8; 5] },
    Input { paddle_dir: i8 },  // -1 up, 0 stop, 1 down
    Ping { t_ms: u32 },
}
```

**Server → Client (S2C):**

```rust
enum S2C {
    Welcome { player_id: u8 },  // 0 = left, 1 = right
    GameState {
        tick: u32,
        t_ms: u32,
        ball_x, ball_y,
        ball_vx, ball_vy,
        paddle_left_y,
        paddle_right_y,
        score_left, score_right,
    },
    GameOver { winner: u8 },
    Pong { t_ms: u32 },
}
```

## Testing

### Automated Tests

```bash
npm run test              # All tests
cargo test --package game_core  # Core game logic
cargo test --package proto      # Protocol serialization
```

### Manual Testing

See `TEST-PLAN.md` for detailed test procedures covering:

- Match creation and joining
- Paddle movement and bounds
- Ball physics and collision
- Scoring and win conditions
- Network synchronization
- Performance (60 fps/60 Hz targets)

### Local vs Production

**Local** (`npm run dev`):

- Uses Miniflare to simulate Cloudflare Workers
- WebSockets work locally
- No rate limits
- State in `.wrangler/state/` (delete to reset)

**Production** (`npm run deploy`):

- Deploys to Cloudflare Workers
- Real Durable Objects
- Subject to Cloudflare rate limits (free tier: 100k requests/day)
- View logs: `npm run logs`

## Troubleshooting

### Build Issues

- Ensure `wasm-pack` is installed: `cargo install wasm-pack`
- Try cleaning: `rm -rf target/ client_wasm/worker/pkg/ lobby_worker/worker/pkg/`

### Local Dev Server Issues

- Port 8787 in use: Kill the process or change port in `wrangler.toml`
- Build failed: Run `npm run build` and check for errors
- Reset state: Delete `.wrangler/state/` directory

### Deployment Issues

- Not logged in: `npx wrangler whoami` (then `npx wrangler login`)
- Rate limits: Use local development or upgrade to paid plan ($5/mo)
- Check logs: `npm run logs`

### Gameplay Issues

- Paddles not moving: Check browser console for WebSocket errors
- Ball not visible: Hard refresh (Cmd+Shift+R) to clear cache
- Lag: Server runs at 60 Hz, but network latency may vary

## Performance

**Client:**

- Target: 60 fps
- Lightweight rendering (paddles + ball only)
- Works on mobile devices

**Server:**

- Fixed tick rate: 60 Hz (16.67ms)
- Snapshot broadcast: 60 Hz to all clients
- Scales to 2 players per match (no bots)

## Cost Optimization (Cloudflare)

The game is configured to minimize costs:

**Free Tier (Development):**

- Input rate: 30 Hz with coalescing
- Server tick: 60 Hz (can reduce to 5 Hz if hitting limits)
- Alarms stop when no clients connected
- Capacity: ~10-12 full matches per day

**Paid Tier ($5/mo base):**

- Light usage (10 matches/day): ~$5-6/month
- Medium usage (50 matches/day): ~$6-7/month
- Heavy usage (200 matches/day): ~$10-15/month

**Monitor usage:**

- Dashboard: https://dash.cloudflare.com
- Logs: `npm run logs`

## Documentation

- **README.md** (this file) - Complete overview and quick start
- **SPEC.md** - Detailed game specification and architecture
- **TEST-PLAN.md** - Comprehensive manual test procedures

## Future Enhancements

Potential additions:

- AI bot opponent (single player)
- Power-ups (speed boost, larger paddle)
- Different game modes (time limit, first to X)
- Visual effects (particle trails, screen shake)
- Sound effects
- Mobile touch controls
- Leaderboard

## License

MIT
