# PONG — Technical Specification

> **Purpose**: Technical specification for the multiplayer Pong implementation using Rust + WebGPU (client) and Cloudflare Durable Objects (server).

---

## Game Mechanics

### Arena
- Fixed size: 32 units wide × 24 units tall
- Paddles on left and right edges
- Top and bottom walls (ball bounces)
- No walls on left/right (scoring zones)

### Paddles
- Size: 1 unit wide × 4 units tall
- Position: X fixed at edges (1 for left, 31 for right)
- Movement: Up/Down at 8 units/second
- Constrained to arena bounds (Y: 2 to 22)

### Ball
- Size: 0.5 unit radius (circle)
- Starting velocity: 8 units/second at random angle
- Bounces off paddles and walls
- Increases speed slightly on each paddle hit (1.05x, max 16 u/s)
- Resets to center after scoring

### Scoring
- Left player scores if ball exits right edge (x > 32)
- Right player scores if ball exits left edge (x < 0)
- First to 11 points wins
- Match ends, winner displayed

### Physics
- Simple AABB collision detection
- Ball bounces reflect velocity: `vel.y = -vel.y` for walls
- **Paddle physics**: Ball trajectory affected by:
  - **Hit position**: Top of paddle deflects ball upward, bottom deflects downward
  - **Paddle movement**: Moving paddle adds 30% of its velocity to the ball
  - Maximum deflection angle: ~45 degrees
- Speed increases on each paddle hit (1.05x multiplier, max 16 u/s)

---

## Technical Architecture

### Stack
- **Client**: Rust → WASM, wgpu (WebGPU)
- **Server**: Rust Durable Object (Cloudflare Workers)
- **ECS**: `hecs` in `game_core` (deterministic)
- **Serialization**: `postcard` binary format
- **Networking**: WebSocket (binary)

### Workspace Structure
```
iso/
  game_core/      # hecs ECS, paddle/ball components & systems
  proto/          # C2S/S2C messages
  client_wasm/    # wgpu renderer, input handling, WebSocket
  server_do/      # Durable Object: WebSocket hub, game loop
  lobby_worker/   # /create /join/:code endpoints, static assets
```

---

## Network Protocol

### Transport
- **WebSocket** (binary via postcard)
- Fixed tick rate: **60 Hz** (16.67ms per tick) for simulation
- Server is authoritative
- Broadcast game state at **20 Hz** (every 3 ticks) to reduce costs

### Messages

**C2S (Client to Server)**
```rust
enum C2S {
    Join { code: [u8; 5] },
    Input { player_id: u8, paddle_dir: i8 },  // -1 = up, 0 = stop, 1 = down
    Ping { t_ms: u32 },
}
```

**S2C (Server to Client)**
```rust
enum S2C {
    Welcome { player_id: u8 },  // 0 = left, 1 = right
    GameState {
        tick: u32,
        ball_x: f32,
        ball_y: f32,
        ball_vx: f32,
        ball_vy: f32,
        paddle_left_y: f32,
        paddle_right_y: f32,
        score_left: u8,
        score_right: u8,
    },
    GameOver { winner: u8 },  // 0 = left, 1 = right
    Pong { t_ms: u32 },
}
```

---

## ECS Layout (hecs)

### Components
```rust
Paddle { player_id: u8, y: f32 }
Ball { pos: Vec2, vel: Vec2 }
PaddleIntent { dir: i8 }  // -1 up, 0 stop, 1 down
```

### Resources
```rust
Time { dt: f32, now: f32 }
GameMap { width: f32, height: f32 }
Score { left: u8, right: u8 }
Config {
    arena_width: f32,
    arena_height: f32,
    paddle_width: f32,
    paddle_height: f32,
    paddle_speed: f32,
    ball_radius: f32,
    ball_speed_initial: f32,
    ball_speed_max: f32,
    ball_speed_increase: f32,
    winning_score: u8,
}
Events { left_scored, right_scored, ball_hit_paddle, ball_hit_wall }
NetQueue { inputs: Vec<PaddleInputEvent> }
```

### System Schedule (Deterministic Order)
1. **IngestInputs** → Apply paddle movement intents from network queue
2. **MovePaddles** → Update paddle Y positions (clamped to arena)
3. **MoveBall** → Update ball position based on velocity
4. **CheckCollisions** → Ball vs paddles, ball vs walls (bounce logic)
5. **CheckScoring** → Detect if ball exited left/right edge
6. **ResetBall** → If scored, reset ball to center with random direction

---

## Rendering (WebGPU)

### Camera
- Orthographic 2D projection
- View entire arena (0, 0) to (32, 24)
- No camera movement

### Meshes
- **Paddle**: Rectangle (0.8 × 4 units)
- **Ball**: Circle (0.5 unit radius)
- Simple colored shapes, no textures

### Render Pipeline
- Multi-pass rendering with ping-pong textures
- Motion blur trails for ball and paddles
- Client-side interpolation for smooth 120 fps rendering
- Instanced rendering for paddles
- Vertex shader: 2D transform (x, y, scale_x, scale_y)
- Fragment shader: Solid color output with trail accumulation

### Performance
- Target: 120 fps rendering with fixed 60 Hz simulation
- Extremely lightweight (< 100 vertices total)
- Trail effects using ping-pong texture accumulation (optional)
- Responsive layout adapts to screen size
- Touch controls for mobile devices

---

## Server (Durable Object)

### Responsibilities
- WebSocket hub: Track 2 clients (left/right players)
- Game simulation: Run fixed 60 Hz loop
- Broadcast game state to both clients
- Handle scoring and win conditions
- Idle timeout after 1 minute of inactivity

### Lifecycle
1. **Creation**: On first `/create` request, mint 5-char code
2. **Player Join**: Client connects via WebSocket, assigned player_id (0 or 1)
3. **Game Loop**: 60 Hz alarms, run `game_core::step`, broadcast state
4. **Game End**: Winner reaches 11 points, send `GameOver` message
5. **Idle Timeout**: Stop alarms when no clients connected

### Persistence
- Match state is in-memory only
- No persistent storage (could be added for leaderboard)

---

## Client Prediction (Future)

Currently not implemented. When added:
- Client runs local `game_core` simulation
- Tag inputs with sequence numbers
- Apply server snapshots (reconciliation)
- Rewind and replay unacked inputs
- Interpolate opponent paddle for smoothness

---

## Performance Targets

- **Client**: 120 fps rendering target with client-side interpolation (fixed 60 Hz simulation)
- **Server**: 60 Hz tick rate (16.67ms per tick) for physics accuracy
- **Network**: 20 Hz state broadcasts (every 3 ticks) to reduce costs
- **Bandwidth**: ~1.7KB/s per client (20 state updates/sec)

---

## Acceptance Criteria

- ✅ Two players can join by 5-character code
- ✅ Paddles move smoothly with keyboard input
- ✅ Ball bounces correctly off paddles and walls
- ✅ Scoring works correctly (left/right edge detection)
- ✅ Game ends at 11 points, winner displayed
- ✅ Stable 60 Hz server ticks
- ✅ Client renders at 120 fps with 60 Hz simulation

---

## Future Extensions

- **AI Bot**: Single player mode with bot opponent
- **Client Prediction**: Instant-feeling controls with server correction
- **Power-ups**: Speed boost, larger paddle, multi-ball
- **Game Modes**: Time limit, first to X points, best of N
- **Visual Effects**: Particle trails, screen shake, bloom
- **Sound Effects**: Ball hit, score, win
- **Leaderboard**: Persistent storage of match results

---

**Last Updated**: 2025-11-11  
**Status**: Core implementation complete with paddle physics, trail effects, responsive layout, and mobile touch controls
