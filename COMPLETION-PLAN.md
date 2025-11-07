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

**M3 - Client WebGPU**: **~60% Complete**

**✅ Completed:**
- ✅ WebGPU surface initialization (wgpu 24.0)
- ✅ Isometric camera (pitch ~35°, yaw offset)
- ✅ Basic rendering pipeline (forward pass, Lambert lighting)
- ✅ Mesh generation (sphere, cube, ground quad)
- ✅ Light buffers (SSBO for up to 8 point lights)
- ✅ Instance buffer infrastructure
- ✅ Game state tracking (players, bolts, pickups)
- ✅ WebSocket message handling (`handle_s2c_message`)

**⏳ Current Blocker:**
- 🔴 **WebGPU render pipeline warnings** - Invalid pipeline preventing rendering
- ⏳ Game entities not visible on screen (pipeline issue)

**⏳ Remaining:**
- ⏳ Fix WebGPU pipeline initialization
- ⏳ Verify entity rendering (players as spheres, bolts, blocks)
- ⏳ Input handling (W/S/A/D, 1/2/3, Q/E/R)
- ⏳ Client prediction (local simulation)
- ⏳ Reconciliation (server correction)
- ⏳ Bloom post-processing
- ⏳ Mobile controls (touch sliders, buttons)
- ⏳ Performance optimization (60fps target)

### ❌ Not Started

**M4 - Bots**: **0% Complete**
- ❌ A* pathfinding
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

### 1. Fix WebGPU Rendering Pipeline (CRITICAL - Current Blocker)
**Goal**: Get entities visible on screen

**Tasks:**
- [ ] Debug WebGPU pipeline warnings (invalid render pipeline)
- [ ] Verify pipeline creation and binding
- [ ] Check shader compilation
- [ ] Verify instance buffer data is correct
- [ ] Test rendering of at least one player entity
- [ ] Verify camera matrices are correct

**Estimated Time**: 2-4 hours

### 2. Complete Basic Rendering (M3 Core)
**Goal**: See all game entities rendered correctly

**Tasks:**
- [ ] Render players as spheres (with eyeball texture placeholder)
- [ ] Render bolts as emissive spheres
- [ ] Render blocks/walls as cubes
- [ ] Render pickups as floating orbs
- [ ] Verify all entities update from snapshots

**Estimated Time**: 4-6 hours

### 3. Input Handling (M3 Core)
**Goal**: Player can control their tank

**Tasks:**
- [ ] Desktop controls (W/S/A/D movement, 1/2/3 bolt, Q/E/R shield)
- [ ] Send input messages via WebSocket (`prepare_input`)
- [ ] Verify inputs reach server
- [ ] Test movement in browser

**Estimated Time**: 2-3 hours

### 4. Client Prediction (M3 Core)
**Goal**: Responsive controls with server authority

**Tasks:**
- [ ] Run local `game_core` simulation
- [ ] Tag inputs with sequence numbers
- [ ] Apply server snapshots (reconciliation)
- [ ] Rewind and replay unacked inputs
- [ ] Interpolate other players

**Estimated Time**: 6-8 hours

### 5. Bloom Post-Processing (M3 Polish)
**Goal**: Visual polish for bolts

**Tasks:**
- [ ] HDR render target
- [ ] Threshold pass
- [ ] Downsample chain
- [ ] Separable blur
- [ ] Composite add

**Estimated Time**: 4-6 hours

### 6. Mobile Controls (M3 Polish)
**Goal**: Playable on mobile devices

**Tasks:**
- [ ] Touch slider for movement (forward/back)
- [ ] Touch slider for turning (left/right)
- [ ] Button UI for bolt levels (1/2/3)
- [ ] Button UI for shield levels (Q/E/R)
- [ ] View rotation controls (two-finger twist or button)

**Estimated Time**: 4-6 hours

### 7. Bots (M4)
**Goal**: Populate matches with AI players

**Tasks:**
- [ ] A* pathfinding on grid
- [ ] Bot state machine implementation
- [ ] Aim prediction with lead calculation
- [ ] Shield reaction logic
- [ ] Bot lifecycle management
- [ ] Difficulty tuning

**Estimated Time**: 12-16 hours

### 8. Polish & Operations (M5)
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

**Minimum Viable Product (MVP)**: ~20-30 hours
- Fix rendering pipeline
- Complete basic rendering
- Input handling
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

1. **Fix WebGPU pipeline** (blocker) → 2-4h
2. **Complete rendering** → 4-6h
3. **Input handling** → 2-3h
4. **Client prediction** → 6-8h
5. **Basic bots** → 8-10h

**Total MVP**: ~22-31 hours

---

## Acceptance Criteria (from SPEC.md)

**For v1 Release:**
- ✅ Create/join by code
- ⏳ 2+ human clients + bots
- ⏳ Stable 20-30 Hz snapshots
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
- WebGPU pipeline issues (current blocker)
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

**Immediate Priority**: Fix WebGPU render pipeline warnings and get at least one entity visible on screen.

**Success Criteria**: 
- No WebGPU warnings in console
- At least one player sphere visible and moving
- Camera follows/isometric view working

---

**Last Updated**: 2025-11-07
**Status**: M2 complete, M3 ~60% complete, blocked on WebGPU pipeline

