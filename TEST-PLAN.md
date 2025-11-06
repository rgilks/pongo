# ISO Test Plan

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
- ⏳ Game entity rendering (players as spheres, bolts, blocks)
- ⏳ Bloom post-processing
- ⏳ Mobile and desktop controls
- ⏳ 60fps target (30fps acceptable on mid-range phones)

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
