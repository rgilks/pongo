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

### M2: DO + Net (To be implemented)

- Create match with 5-char code
- Join match by code
- Multiple clients sync via WebSocket
- Client prediction works correctly
- Reconciliation handles server corrections

### M3: Client WebGPU (To be implemented)

- 3D isometric rendering
- Bloom post-processing
- Point lights from bolts
- Mobile and desktop controls
- 60fps target (30fps acceptable on mid-range phones)

### M4: Bots (To be implemented)

- Bots navigate with A*
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
- ⚠️ Pickup collection (edge case)
- ⚠️ Hill scoring (edge case)

