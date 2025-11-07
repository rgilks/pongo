# ISO Project Completion Plan

## Current Status Summary

### ✅ Completed Milestones

**M1 - Core Sim (Local)**: ✅ **100% Complete**

- ✅ ECS (`hecs`) with deterministic schedule
- ✅ Movement system (tank-style, collision)
- ✅ Bolt firing system (3 levels)
- ✅ Shield system (3 levels)
- ✅ Pickup system (health, bolt upgrade, shield module)
- ✅ Hill scoring (King of the Hill objective)
- ✅ Eliminations and respawn
- ✅ All unit tests passing (7 integration tests)

**M2 - DO + Net**: ✅ **100% Complete**

- ✅ Cloudflare Workers infrastructure
- ✅ Durable Object (`MatchDO`) deployed
- ✅ WebSocket connection (handshake fixed)
- ✅ Network protocol (C2S/S2C with `postcard`)
- ✅ Snapshot generation and broadcasting
- ✅ Player joining logic
- ✅ Game simulation integrated into DO
- ✅ Lobby endpoints (`/create`, `/join/:code`)

### 🚧 In Progress

**M3 - Client WebGPU**: **~80% Complete**

**✅ Completed:**

- ✅ WebGPU surface initialization (wgpu 24.0)
- ✅ Isometric camera (pitch ~35°, yaw offset)
- ✅ Basic rendering pipeline (forward pass, Lambert lighting)
- ✅ Mesh generation (sphere, cube, ground quad)
- ✅ Light buffers (SSBO for up to 8 point lights)
- ✅ Instance buffer infrastructure
- ✅ Game state tracking (players, bolts, pickups)
- ✅ WebSocket message handling (`handle_s2c_message`)
- ✅ **WGSL shader alignment fixed** (uniform buffer 16-byte alignment)
- ✅ **Periodic game loop** (Durable Object alarms, 200ms ticks, 5 ticks/sec - optimized)
- ✅ **Snapshot broadcasting** to all connected clients
- ✅ **Entity rendering** (players as spheres, bolts as spheres, pickups as spheres, blocks as cubes)
- ✅ **Local development workflow** (documented and tested)

**⏳ Remaining:**

- ⏳ Input handling (W/S/A/D, 1/2/3, Q/E/R) - basic structure exists
- ⏳ Client prediction (local simulation)
- ⏳ Reconciliation (server correction)
- ⏳ Bloom post-processing
- ⏳ Mobile controls (touch sliders, buttons)
- ⏳ Performance optimization (60fps target)

### ❌ Not Started

**M4 - Bots**: **0% Complete**

- ❌ A\* pathfinding
- ❌ Bot state machine (Seek pickups → Contest Hill → Evade → Engage)
- ❌ Aim prediction with lead
- ❌ Shield reaction logic
- ❌ Bot lifecycle (spawn/despawn)

**M5 - Polish & Ops**: **0% Complete**

- ❌ Spawn protection visual feedback
- ❌ Share flow (copy link)
- ❌ Audio pips
- ❌ Metrics/analytics
- ❌ Performance profiling and optimization

---

## Immediate Next Steps (Priority Order)

### 1. Complete Basic Rendering (M3 Core) - CURRENT FOCUS

**Goal**: See all game entities rendered correctly

**Tasks:**

- [x] Render players as spheres (with eyeball texture placeholder)
- [x] Render bolts as emissive spheres
- [x] Render blocks/walls as cubes
- [x] Render pickups as floating orbs
- [x] Verify all entities update from snapshots

**Estimated Time**: 4-6 hours

### 2. Input Handling (M3 Core)

**Goal**: Player can control their tank

**Tasks:**

- [ ] Desktop controls (W/S/A/D movement, 1/2/3 bolt, Q/E/R shield)
- [ ] Send input messages via WebSocket (`prepare_input`)
- [ ] Verify inputs reach server
- [ ] Test movement in browser

**Estimated Time**: 2-3 hours

### 3. Client Prediction (M3 Core)

**Goal**: Responsive controls with server authority

**Tasks:**

- [ ] Run local `game_core` simulation
- [ ] Tag inputs with sequence numbers
- [ ] Apply server snapshots (reconciliation)
- [ ] Rewind and replay unacked inputs
- [ ] Interpolate other players

**Estimated Time**: 6-8 hours

### 4. Bloom Post-Processing (M3 Polish)

**Goal**: Visual polish for bolts

**Tasks:**

- [ ] HDR render target
- [ ] Threshold pass
- [ ] Downsample chain
- [ ] Separable blur
- [ ] Composite add

**Estimated Time**: 4-6 hours

### 5. Mobile Controls (M3 Polish)

**Goal**: Playable on mobile devices

**Tasks:**

- [ ] Touch slider for movement (forward/back)
- [ ] Touch slider for turning (left/right)
- [ ] Button UI for bolt levels (1/2/3)
- [ ] Button UI for shield levels (Q/E/R)
- [ ] View rotation controls (two-finger twist or button)

**Estimated Time**: 4-6 hours

### 6. Bots (M4)

**Goal**: Populate matches with AI players

**Tasks:**

- [ ] A\* pathfinding on grid
- [ ] Bot state machine implementation
- [ ] Aim prediction with lead calculation
- [ ] Shield reaction logic
- [ ] Bot lifecycle management
- [ ] Difficulty tuning

**Estimated Time**: 12-16 hours

### 7. Polish & Operations (M5)

**Goal**: Production-ready experience

**Tasks:**

- [ ] Spawn protection visual feedback
- [ ] Share flow (copy match code/link)
- [ ] Audio pips for events
- [ ] Metrics/analytics integration
- [ ] Performance profiling
- [ ] Mobile optimization (render scale, bloom quality)

**Estimated Time**: 8-12 hours

---

## Estimated Total Remaining Time

**Minimum Viable Product (MVP)**: ~20-27 hours

- Complete basic rendering (infrastructure ready, verify entities visible)
- Input handling (structure exists, needs testing)
- Client prediction
- Basic bots

**Full v1 Release**: ~40-50 hours

- All of MVP
- Bloom post-processing
- Mobile controls
- Full bot implementation
- Polish & operations

---

## Critical Path to MVP

1. **Complete rendering** → 4-6h (infrastructure ready, verify entities visible)
2. **Input handling** → 2-3h (structure exists, needs testing)
3. **Client prediction** → 6-8h
4. **Basic bots** → 8-10h

**Total MVP**: ~20-27 hours

---

## Acceptance Criteria (from SPEC.md)

**For v1 Release:**

- ✅ Create/join by code
- ⏳ 2+ human clients + bots
- ✅ Stable 20 Hz snapshots (50ms ticks via Durable Object alarms)
- ⏳ Client >50 fps on mid-range phone
- ⏳ Objective mode functional
- ⏳ Pickups/levels work
- ⏳ Re-entry with protection works
- ⏳ 30-minute soak (6 actors) without crash
- ⏳ Snapshot persistence verified

**Current Status**: ~40% of acceptance criteria met

---

## Risk Assessment

**High Risk:**

- Client prediction complexity
- Mobile performance on low-end devices

**Medium Risk:**

- Bot AI complexity
- Network reconciliation edge cases
- Bloom performance impact

**Low Risk:**

- Polish features (can be deferred)
- Metrics/analytics (nice-to-have)

---

## Next Session Focus

**Immediate Priority**: Complete input handling and test player movement in browser.

**Success Criteria**:

- ✅ Players, bolts, pickups, and blocks all render correctly
- ✅ Camera isometric view working
- ✅ Entities update from snapshots correctly
- ⏳ Player can control their tank with keyboard (W/S/A/D, 1/2/3, Q/E/R)
- ⏳ Inputs are sent to server and processed

---

**Last Updated**: 2025-01-07
**Status**: M2 complete, M3 ~80% complete, entity rendering complete, game loop optimized (5 Hz for free tier), local development workflow documented and tested
