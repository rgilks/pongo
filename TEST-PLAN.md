# ISO Test Plan

## Local Development Setup

### Prerequisites

- Build the project: `npm run build`
- Ensure Wrangler is installed: `npx wrangler --version`
- Login to Cloudflare (one-time): `npx wrangler login`

### Starting Local Server

```bash
# Build client and server WASM
npm run build

# Start local dev server
npm run dev
# Server starts at http://localhost:8787
```

### Local Testing Benefits

- ✅ No rate limits - unlimited testing
- ✅ Faster iteration - instant code changes
- ✅ Better debugging - terminal logs
- ✅ Isolated from production

### Verified Local Functionality

- ✅ Match creation (`/create` endpoint)
- ✅ Match joining (`/join/:code` endpoint)
- ✅ WebSocket connection establishment
- ✅ Client WASM initialization
- ✅ WebGPU rendering pipeline
- ✅ Durable Objects via Miniflare

### Troubleshooting

**Common Issues:**

- **Server won't start**

  - Ensure `npm run build` completed successfully
  - Check for compilation errors in terminal
  - Verify Wrangler is installed: `npx wrangler --version`

- **WebSocket errors**

  - Check terminal for detailed error messages
  - Verify Durable Object is receiving requests
  - Check browser console for client-side errors

- **Port 8787 in use**

  - Kill process: `lsof -ti:8787 | xargs kill` (macOS/Linux)
  - Or use different port: `npx wrangler dev --port 8788`

- **Reset local state**

  - Delete `.wrangler/state/` directory to clear all local Durable Object state
  - Useful when testing match creation/joining

- **Build errors**
  - Run `cargo clean` and rebuild
  - Check Rust version: `rustc --version` (should be stable)
  - Verify wasm-pack: `wasm-pack --version`

## Manual Tests (Browser Automation)

### M1: Core Sim (Local)

✅ **Movement**

- Player moves forward/backward with W/S
- Player turns left/right with A/D
- Collision with walls prevents movement

✅ **Combat**

- Fire bolts with 1/2/3 keys
- Bolts travel and hit players
- Shield blocks bolts in frontal arc
- Energy drains when firing/shielding

✅ **Pickups**

- Health orbs restore damage
- Bolt upgrades increase max level
- Shield modules unlock/upgrade shield

✅ **Hill Scoring**

- Solo player in hill earns points
- Contested hill awards no points
- First to 100 points wins

✅ **Eliminations**

- 3 damage = elimination
- Respawn after 2s delay
- Spawn shield protects for 0.5s

### M2: DO + Net: ✅ Complete

- ✅ Create match with 5-char code
- ✅ Join match by code
- ⏳ Multiple clients sync via WebSocket (infrastructure ready, needs client-side)
- ⏳ Client prediction works correctly (pending client implementation)
- ⏳ Reconciliation handles server corrections (pending client implementation)

### M3: Client WebGPU: 🚧 In Progress

- ✅ WebGPU surface initialization
- ✅ Isometric camera (pitch ~35°, yaw offset support)
- ✅ Basic rendering pipeline (forward pass, lambert lighting)
- ✅ Mesh generation (sphere, cube, ground quad)
- ✅ Light buffers (SSBO for up to 8 point lights)
- ✅ WGSL shader alignment fixed (uniform buffer requirements)
- ✅ Periodic game loop running (200ms ticks, 5 ticks/sec - optimized for free tier)
- ✅ Snapshot broadcasting to clients
- ✅ **Game entity rendering** (players as spheres, bolts as spheres, pickups as spheres, blocks as cubes)
- ⏳ Bloom post-processing
- ⏳ Mobile and desktop controls
- ⏳ 60fps target (30fps acceptable on mid-range phones)

**Local Testing:**

1. Start local server: `npm run dev`
2. Open `http://localhost:8787` in browser
3. Create/join a match
4. Verify all entities render:
   - Ground quad (gray)
   - Players (red spheres)
   - Bolts (colored spheres by level)
   - Pickups (colored spheres by type)
   - Blocks/walls (gray and brown cubes)
5. Test WebSocket connection works locally

### M4: Bots (To be implemented)

- Bots navigate with A\*
- Bots collect pickups
- Bots contest hill
- Bots react to incoming bolts
- Bot difficulty adjustable

### M5: Polish & Ops (To be implemented)

- Spawn protection visual feedback
- Share flow works
- Audio pips play
- Metrics tracked
- Performance targets met

## Automated Tests

Run with `cargo test --workspace`:

- ✅ Movement system
- ✅ Bolt firing
- ✅ Energy drain
- ✅ Health damage
- ✅ Eliminations
- ✅ Pickup collection
- ✅ Hill scoring
