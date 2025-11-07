# ISO — Full Specification (v0.2)

> **Purpose**: Ship a mobile-friendly (PWA), code-to-join, isometric arena shooter called **ISO**, built engine‑free to learn **Rust + WebGPU** (client) and **Cloudflare Durable Objects** (server). Deterministic **ECS (hecs)** simulation is shared by client (prediction) and server (authority). One **Durable Object per game code**. No login.

---

## 1. Product Goals & Non‑Goals

**Goals**

* Fast, learnable demo of Rust/WASM/WebGPU + Rust Durable Objects.
* Low-friction play: create/join by **5‑char code**, shareable link. No accounts.
* **Tank‑style** controls, 2D simulation with **3D isometric** presentation.
* Satisfying **glow/lighting** on shots with lightweight bloom/point lights.
* **Bots** populate matches; **pickups** and **objective (King of the Hill)** create engagement.

**Non‑Goals v1**

* No complex PBR/shadows; no physics‑heavy ricochets; no full game engine.
* No text chat, cosmetics economy, or persistent progression.

---

## 2. Core Game Loop

1. Player joins match by code; picks from preset **name+eyeball** avatar.
2. Navigate arena, collect pickups, fight others with **Bolts** and **Shield**.
3. Contest the **Hill** to earn points; first to target or top at timer wins.
4. On elimination (3 damage), **Re‑enter** after short delay.

---

## 3. Mechanics & Tuning (v1 defaults)

**Movement (tank‑style)**

* Turn rate: **210°/s**; forward/back speed: **6.5 u/s** (player radius **0.6 u**).
* 2D plane simulation; circle vs AABB collisions.

**Resources**

* **Energy** 0–100; regen **15/s**; spent on shots & shield.
* **Health** as **3 damage slots** (0..3). At 3 → eliminated.

**Weapon — Bolt** (projectile fired along yaw)

* Cooldown **150 ms**. Lifetime **1.6 s**.
* Levels (costE / speed / damage / radius):

  * **L1:** 10 / 10 / 1 / 0.25
  * **L2:** 20 / 13 / 2 / 0.30
  * **L3:** 35 / 16 / 3 / 0.35

**Shield** (frontal arc ~120°)

* Levels S1/S2/S3 drain **8/16/28 E per second**; **max up 0.6 s**; **cooldown 0.4 s**.
* On hit: `damage = max(0, BoltLevel − ShieldLevel)` at contact angle. (L3 vs none = 3.)

**Starting kit**

* Start with **Bolt L1 only**, **no Shield**.

**Pickups**

* **Health Orb**: restore 1 damage slot (max 3).
* **Bolt Upgrade**: raise bolt **max level** up to L3.
* **Shield Module**: unlock/raise shield **max level** up to S3.
* **Spawn pads**: typed pads in map (8–12). Each pad holds one item; **respawn 8–16 s (± jitter)**; despawn stale after 20 s.

**Objective — King of the Hill**

* One circular Hill (r=3 u) centered; optional rotation among 3 presets every 60 s.
* Solo occupant earns **+1 point/sec**; contested → no gain.
* **Win**: first to **100 points** or highest after **5 min**.
* If Objective **OFF**: FFA; most eliminations at **5 min**.

**Respawn**

* Re‑enter after **2 s** at safe spawn; **spawn shield S2** for **0.5 s** (no firing during).

---

## 4. Player Experience & UX

**Flows**

* **Home**: Create | Join.
* **Create**: server mints 5‑char code (Crockford Base32, no vowels/ambiguous). Show link + **Share**.
* **Join**: type code or open link. Choose **preset name & eyeball** (or Random).
* **Lobby**: show players/bots; toggles: Objective ON/OFF; Start.

**Controls**

* **Mobile**: left slider = Forward/Back; right slider = Turn L/R; buttons **BOLT L1–L3**, **SHIELD S1–S3**; two‑finger twist or 90° rotate button; **Share**.
* **Desktop**: `W/S` move, `A/D` turn; `1/2/3` bolt, `Q/E/R` shield; `Space` hold shield; wheel or `Z/X` rotate view.

**Visual Language**

* Player = **eyeball** sphere with iris/pupil; shield color ring; bolt is emissive orb; pickups float & pulse.

---

## 5. AI Bots (server‑side)

**Purpose**: Fill rooms to a target population (e.g., 6 actors).

**Behavior**

* **Nav**: grid‑based **A*** on 1×1 grid. Precompute connectivity.
* **State machine**: *Seek pickups* → *Contest Hill* → *Evade bolt* → *Engage*.
* **Aim**: impact lead prediction (solve t for projectile speed); noise by difficulty.
* **Energy use**: fire highest level affordable; raise shield if incoming bolt TTI < 250 ms & inside arc.
* **Priorities**: Bolt/Shield upgrades > Health (if damaged) > Hill (if near) > Chase target.

**Difficulty knobs**

* Reaction delay (ms), aim error (deg), fire min gap, retreat threshold, hill obsession.

**Lifecycle**

* Spawn when humans < target; remove oldest bot at round end or safe window when humans join.

---

## 6. Technical Architecture

**Stack**

* **Client**: Rust → WASM, **wgpu** + WGSL, no engine.
* **Server**: Rust **Durable Object** (Workers `worker` crate). Event‑driven soft‑tick.
* **ECS**: **`hecs`** in `game_core` (single‑threaded, deterministic schedule).
* **Serialization**: `postcard` + quantization (i16/u16/u8).
* **Hosting**: Cloudflare Workers/DO; static assets served by Worker (or R2).

**Workspace**

```
iso/
  game_core/      # hecs ECS, systems, components, params, ai
  proto/          # C2S/S2C, quantization, versioning
  client_wasm/    # wgpu renderer, input, prediction, WS
  server_do/      # Durable Object Match: WS hub, step, storage, bots
  lobby_worker/   # /create /join/:code, serves client
  assets/         # eyeball textures, meshes, sfx
  wrangler.toml
```

**Cloudflare**

```toml
# wrangler.toml
name = "iso"
main = "worker/index.js"
compatibility_date = "2024-01-01"

[durable_objects]
bindings = [{ name = "MATCH", class_name = "MatchDO" }]

[[migrations]]
tag = "v1"
new_sqlite_classes = ["MatchDO"]
```

---

## 7. Netcode & Synchronization

**Transport**: WebSocket (binary). Clients send **inputs only**; server is authoritative.

**Soft‑tick loop (server DO)**

* Step simulation on **50 ms alarm** (20 ticks/sec) using Durable Object alarms.
* Consume real `dt` in **fixed micro‑steps** (e.g., 8–12 ms) → stable physics.
* Broadcast compact **Snapshot** (20/s via alarms). Clients ACK.
* Alarm starts automatically when first player joins.

**Client prediction**

* Run same `game_core` schedule locally. Tag inputs `(seq, t_client_ms)`.
* On Snapshot `(last_seq_ack)`: rewind to acked state, reapply unacked inputs.
* Others rendered with ~**120 ms** interpolation buffer.

**Lag compensation (simple)**

* On bolt vs shield, sample defender orientation closest to hit timestamp window.

---

## 8. Data Protocol (proto)

**Quantization**

* World bounds ±32 u → position/velocity **i16** (scaled); yaw **u16** (0..65535 → 0..360°);
* Energy **u16** (0..1000 → 0..100.0).

**C2S**

```
Join { code:[u8;5], avatar:u8, name_id:u8 }
Input{ seq:u32, t_ms:u32, thrust_i8:i8, turn_i8:i8, bolt:u8(0..3), shield:u8(0..3) }
Ping { t_ms:u32 }
Ack  { snapshot_id:u32 }
```

**S2C**

```
Welcome{ player_id:u16, params_hash:u32, map_rev:u16 }
Snapshot{ id:u32, tick:u32, t_ms:u32, last_seq_ack:u32,
          players:[PlayerP], bolts:[BoltP], pickups:[PickupP],
          hill_owner:Option<u16>, hill_progress_u16:u16 }
Eliminated{ player_id:u16 }
Ended{ standings:[(player_id:u16, points:u16)] }
```

**PlayerP**: `{ id:u16, pos_q:[i16;2], vel_q:[i16;2], yaw_q:u16, bolt_max:u8, shield_max:u8, hp:u8, energy_q:u16, flags:u8 }`

**BoltP**: `{ id:u16, pos_q:[i16;2], vel_q:[i16;2], rad_q:u8, level:u8, owner:u16 }`

---

## 9. ECS (hecs) Layout & Deterministic Schedule

**Key Components**

* `Transform2D{ pos:Vec2, yaw:f32 }`, `Velocity2D{ vel:Vec2 }`
* `Player{ id:u16, avatar:u8, name_id:u8 }`, `Health{ damage:u8 }`, `Energy{ cur:f32 }`
* `Shield{ max:u8, active:u8, t_left:f32, cooldown:f32 }`
* `Bolt{ level:u8, dmg:u8, radius:f32, owner:u16 }`, `Lifetime{ t_left:f32 }`
* `Pickup{ kind:Health|BoltUp|ShieldMod }`, `SpawnPad{ kind, respawn:Range<f32>, t_until:f32 }`
* `HillZone{ center:Vec2, r:f32 }`, `BotBrain{ state, reaction_ms, aim_err_deg, ... }`

**Resources**

* `Params`, `Map{ blocks:Vec<Aabb>, spawns:Vec<Vec2> }`, `Time{ dt, now }`
* `Rng`, `Score{ hill_points:HashMap<PlayerId,u16> }`
* `NetQueue{ inputs:Vec<InputEvent>, acks:... }`, `Config{ objective_on, target_actors, ... }`
* `Events{ spawn_bolt:Vec<...>, apply_damage:Vec<...>, eliminated:Vec<...>, pickup_taken:Vec<...> }`

**Fixed Schedule (order matters)**

1. **IngestInputs** → intents (per player)
2. **BotThink** → intents for bots
3. **ApplyMovementIntent** → turn/thrust → velocity
4. **IntegrateMotion** → pos += vel*dt; circle vs AABB
5. **ShieldUpdate** → start/stop, drain energy, timers
6. **FireBolts** → energy & cooldown checks → `SpawnBolt`
7. **SpawnFromEvents** → create Bolt + Lifetime
8. **BoltsStep** → move; collide vs blocks; expire
9. **ResolveHits** → bolts vs players; compute damage events
10. **ApplyDamage & Eliminations** → update health; queue respawns
11. **PickupsSpawn** → advance pads, spawn item entities
12. **PickupsCollect** → apply effects; remove item
13. **HillScoreTick** → score if uncontested
14. **EnergyRegen**
15. **GC** → despawn expired/lost
16. **SnapshotExtract** → SoA net state; quantize; hash
17. **ApplyCommands**
18. **NetBroadcast** (server) / **Reconcile** (client)

**Determinism Rules**

* Never rely on raw query order; **collect & sort by `Entity::id()`** when needed.
* Use fixed micro‑steps; clamp dt.
* Single RNG resource; only used in allowed systems; server authority wins.

---

## 10. Rendering (engine‑free, wgpu)

**Camera**: isometric (pitch ~35°) + user yaw offset. 3D scene; 2D sim.

**Meshes**: unit sphere (eye/bolt), unit cube (block), ground quad.

**Materials/Pipelines**

* **Forward Pass** (walls/ground/eyes) with up to **N small point lights** from bolts (CPU‑culled, cap 6–8). Simple lambert + small ambient.
* **Emissive Bolts** → bright values into HDR target.
* **Bloom**: threshold → downsample → separable blur (quarter res) → composite add.

**GPU Buffers**

* Camera UBO (matrices, padded). Lights SSBO (`struct Light{ pos, radius, color, intensity }` + `lightCount`). Instance buffers per mesh (packed 2D transform, tint, flags).

**Performance Knobs (mobile)**

* Render scale 0.75×; bloom 0.25×. Cap lights to 6. Keep fragment branches low.

**Optional Phase 3**: Low‑res 2D **lightmap** (128×128) over arena, compute‑blurred, sampled by walls/ground for soft global glow.

---

## 11. Backend (Durable Object)

**Instance per game code**: `MATCH.id_from_name(code)`.

**Responsibilities**

* WS hub: track clients; binary frames; rate‑limit inputs ≤60/s.
* Simulation: run fixed schedule on input/heartbeat.
* Bots: maintain target actor count; despawn on human join.
* Persistence: store compact snapshot every 5 s & major events; idle expire after 10 min (code reusable).
* Safety: reject impossible actions (energy/cooldown), clamp inputs, spawn protection.

---

## 12. Telemetry & Operations

* Workers **Analytics Engine**: joins, RTT, CPU per step, broadcast rate, actors.
* Error logs to Logpush sink.
* Feature flags in DO storage: bots on/off, objective on/off, pickup rates, difficulty.

---

## 13. Testing & QA

**Unit**: systems (movement, hits, pickups, hill scoring) — deterministic assertions.

**Golden snapshots**: scripted input sequences → compare SoA outputs/hashes.

**Soak**: headless bot‑only matches 10k steps; monitor invariants (no NaNs, bounds).

**Net**: latency/jitter/packet‑loss shim in client; validate prediction/reconcile stability.

**E2E**: two browsers + bots; verify win conditions, respawn, persistence.

---

## 14. Performance Targets

* **Client**: 60 fps target (30 fps acceptable) on mid‑range phones.
* **Server (DO)**: step + broadcast < **10 ms** per event burst; snapshots ≤ **350 B** typical.
* **Network**: 20–30 snapshots/s; inputs 30–60/s.

---

## 15. Milestones & Deliverables

**M1 — Core Sim (local)**: ECS (`hecs`), movement/bolts/shield, pickups, hill, eliminations.

**M2 — DO + Net**: WS hub; soft‑tick; snapshots/ACK; persistence; idle expiry; join/create by code.

**M3 — Client WebGPU**: meshes, isometric camera, emissive bolts, bloom; mobile+desktop controls; prediction/reconcile; map rotation.

**M4 — Bots**: A* nav; aim lead; shield reactions; pickup & hill behaviors; difficulty knobs.

**M5 — Polish & Ops**: spawn protection UX, share flow, audio pips, metrics, perf passes.

**Optional M6 — Leaderboards**: D1 tables: matches, players, scores (daily top list).

**Acceptance (v1)**

* Create/join by code; 2+ human clients + bots; stable 20–30 Hz snapshots; client >50 fps mid‑phone.
* Objective mode functional; pickups/levels work; re‑entry w/ protection works.
* 30‑minute soak (6 actors) without crash; snapshot persistence verified.

---

## 16. Config & Tuning (per‑match)

* `max_players` (8), `target_actors` (6), `objective_on` (bool),
* `hill_points_to_win` (100), `match_time_s` (300),
* pickup respawn windows, energy regen, bolt/shield tables, bot difficulty.

---

## 17. Security & Compliance

* No PII; preset names only.
* Server authoritative; input sanitation; spawn protection; rate limits.
* No user‑generated chat/content in v1.

---

## 18. Future Extensions (post‑v1)

* Ricochet bolts; beam alt‑fire; destructible blocks.
* Team modes, cosmetics, progression.
* Spectator & replay (input log + seed).
* Post‑processing: bloom tuning, SSAO lite; animated eyeball avatars.

---

## 19. Appendix: Implementation Notes

* **Uniform alignment**: 256‑byte for UBOs; light arrays in SSBO.
* **Ring buffers**: per‑frame mapped staging for buffer updates.
* **Bind groups**: keep to camera, lights, material/sampler.
* **Determinism**: always sort entity lists by `Entity::id()` before order‑dependent ops.
* **Quant scales**: pos i16 → ±32 u with 1024 steps/u; yaw u16 → 0..2π; energy u16 → 0..1000.
* **Code minting**: Crockford Base32, exclude vowels & ambiguous (0/O,1/I/L,2/Z).

---
