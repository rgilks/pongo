# PONG — Full Specification (v1.0)

> **Purpose**: Build a multiplayer Pong game to demonstrate **Rust + WebGPU** (client) and **Cloudflare Durable Objects** (server) architecture. Deterministic **ECS (hecs)** simulation with client-server networking. One **Durable Object per game code**. No login required.

---

## 1. Product Goals & Non‑Goals

**Goals**

* Simple, learnable demo of Rust/WASM/WebGPU + Rust Durable Objects.
* Low-friction play: create/join by **5‑char code**, shareable link. No accounts.
* Classic Pong mechanics: two paddles, one ball, simple physics.
* Clean rendering with WebGPU.

**Non‑Goals v1**

* No fancy graphics, just simple shapes (rectangles and circle).
* No AI bots (human players only).
* No complex physics, power-ups, or game modes.

---

## 2. Core Game Loop

1. Two players join a match by code.
2. Each player controls a paddle (left or right side).
3. Ball bounces between paddles and top/bottom walls.
4. Player scores when opponent misses the ball.
5. First to 11 points wins.

---

## 3. Game Mechanics

**Arena**

* Fixed size: 32 units wide × 24 units tall
* Paddles on left and right edges
* Top and bottom walls (ball bounces)
* No walls on left/right (scoring zones)

**Paddles**

* Size: 1 unit wide × 4 units tall
* Position: X fixed at edges (1 for left, 31 for right)
* Movement: Up/Down at 8 units/second
* Constrained to arena bounds

**Ball**

* Size: 0.5 unit radius (circle)
* Starting velocity: 8 units/second at random angle
* Bounces off paddles and walls
* Increases speed slightly on each paddle hit (1.05x, max 16 u/s)
* Resets to center after scoring

**Scoring**

* Left player scores if ball exits right edge (x > 32)
* Right player scores if ball exits left edge (x < 0)
* First to 11 points wins
* Match ends, show winner

**Physics**

* Simple AABB collision detection
* Ball bounces reflect velocity: `vel.y = -vel.y` for walls
* Paddle hits reflect and add paddle velocity influence
* No spin or complex physics

---

## 4. Player Experience & UX

**Flows**

* **Home**: Create | Join
* **Create**: server mints 5‑char code. Show link + **Share**
* **Join**: type code or open link
* **Waiting**: show "Waiting for opponent..." until 2 players
* **Game**: play Pong!
* **End**: show winner, option to rematch

**Controls**

* **Desktop**: `Up/Down` arrow keys or `W/S` keys
* **Mobile**: On-screen Up/Down buttons
* Simple and responsive

**Visual**

* Paddles: white rectangles
* Ball: white circle
* Background: black
* Score display at top center
* Clean, classic Pong aesthetic

---

## 5. Technical Architecture

**Stack**

* **Client**: Rust → WASM, **wgpu** (WebGPU), no game engine
* **Server**: Rust **Durable Object** (Cloudflare Workers)
* **ECS**: **`hecs`** in `game_core` (deterministic)
* **Serialization**: `postcard` binary format
* **Hosting**: Cloudflare Workers/DO

**Workspace**

```
pong/
  game_core/      # hecs ECS, paddle/ball components & systems
  proto/          # C2S/S2C messages
  client_wasm/    # wgpu renderer, input handling, WebSocket
  server_do/      # Durable Object: WebSocket hub, game loop
  lobby_worker/   # /create /join/:code endpoints, static assets
  assets/         # (minimal, if any)
```

---

## 6. Netcode & Synchronization

**Transport**: WebSocket (binary)

**Server Loop**

* Fixed tick rate: **60 Hz** (16.67ms per tick)
* Server is authoritative
* Broadcast game state (ball pos, paddle pos, score) to both clients
* Process player inputs (move paddle up/down)

**Client Prediction**

* Optional: predict local paddle movement
* Server state overrides client predictions
* Interpolate opponent paddle for smoothness

---

## 7. Data Protocol

**C2S (Client to Server)**

```rust
enum C2S {
    Join { code: [u8; 5] },
    Input { paddle_dir: i8 },  // -1 = up, 0 = stop, 1 = down
    Ping { t_ms: u32 },
}
```

**S2C (Server to Client)**

```rust
enum S2C {
    Welcome { player_id: u16 },  // 0 = left, 1 = right
    GameState {
        tick: u32,
        ball_pos: [f32; 2],
        ball_vel: [f32; 2],
        paddle_left_y: f32,
        paddle_right_y: f32,
        score_left: u8,
        score_right: u8,
    },
    GameOver { winner: u8 },  // 0 = left, 1 = right
}
```

---

## 8. ECS (hecs) Layout

**Components**

* `Paddle { player_id: u8, y: f32 }`
* `Ball { pos: Vec2, vel: Vec2 }`
* `Score { left: u8, right: u8 }`

**Systems (deterministic order)**

1. **IngestInputs** → apply paddle movement intents
2. **MovePaddles** → update paddle Y positions (clamped to arena)
3. **MoveBall** → update ball position
4. **CheckCollisions** → ball vs paddles, ball vs walls
5. **CheckScoring** → ball exited left/right edge
6. **ResetBall** → if scored, reset to center
7. **ExtractSnapshot** → serialize state for network

**Resources**

* `Time { dt: f32, now: f32 }`
* `Arena { width: f32, height: f32 }`
* `Config { paddle_speed: f32, ball_speed: f32, win_score: u8 }`

---

## 9. Rendering (WebGPU)

**Camera**

* Orthographic 2D camera
* View the entire arena (0, 0) to (32, 24)

**Meshes**

* Paddle: rectangle (1 × 4 units)
* Ball: circle (0.5 unit radius)
* Simple colored shapes, no textures

**Render Pipeline**

* Single pass
* Flat colors (white on black)
* Instanced rendering for paddles

**Performance**

* Extremely lightweight, 60 fps easy on any device
* No bloom, no fancy effects (can be added later)

---

## 10. Durable Object (Server)

**Responsibilities**

* WebSocket hub: track 2 clients (left/right players)
* Game simulation: run fixed 60 Hz loop
* Broadcast game state to both clients
* Handle scoring and win conditions
* Idle timeout after 5 minutes of inactivity

**Persistence**

* Optional: store match results for leaderboard
* Match codes can be reused after timeout

---

## 11. Milestones

**M1 — Core Sim (local)**

* ECS components and systems
* Paddle movement
* Ball physics and collision
* Scoring logic

**M2 — DO + Net**

* Durable Object setup
* WebSocket connections
* Game state broadcasting
* 2-player support

**M3 — Client Rendering**

* WebGPU initialization
* Render paddles and ball
* Handle input (Up/Down keys)
* Display score

**M4 — Polish**

* Win condition and game over screen
* Rematch flow
* Mobile touch controls
* Minor visual improvements

---

## 12. Acceptance Criteria

* Two players can join by code
* Paddles move smoothly with keyboard input
* Ball bounces correctly off paddles and walls
* Scoring works correctly
* Game ends at 11 points
* Stable 60 Hz server ticks
* Client runs at 60 fps

---

## 13. Testing

**Unit Tests**

* Ball collision logic
* Scoring conditions
* Paddle movement bounds

**Integration Tests**

* Two clients connect and play
* Network state synchronization
* Win condition handling

**Manual Tests**

* Play a full game to 11 points
* Test edge cases (ball hitting corner, etc.)
* Test on mobile devices

---

## 14. Future Extensions

* AI bot opponent (single player mode)
* Power-ups (speed boost, larger paddle, etc.)
* Different game modes (first to X, time limit, etc.)
* Visual effects (particle trails, screen shake, etc.)
* Sound effects
* Leaderboard

---

**Last Updated**: 2025-11-10
**Status**: Initial specification for Pong branch
