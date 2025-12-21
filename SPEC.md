# Pongo — Technical Specification

> Technical reference for the WebGPU Pong implementation.

## Architecture

| Layer | Technology | Description |
|-------|------------|-------------|
| Client | Rust → WASM, [wgpu](https://wgpu.rs/) v24 | WebGPU renderer, client prediction |
| Server | Rust Durable Object | Authoritative 60 Hz simulation |
| ECS | [hecs](https://docs.rs/hecs) | Shared game logic |
| Protocol | [postcard](https://docs.rs/postcard) | Binary WebSocket messages |

## Game Constants

```rust
// Arena
ARENA_WIDTH:  32.0    // units
ARENA_HEIGHT: 24.0    // units

// Paddle
PADDLE_WIDTH:  0.8    // units
PADDLE_HEIGHT: 4.0    // units  
PADDLE_SPEED: 12.0    // units/sec

// Ball
BALL_RADIUS:        0.5   // units
BALL_SPEED_INITIAL: 8.0   // units/sec
BALL_SPEED_MAX:    16.0   // units/sec
BALL_SPEED_INCREASE: 1.05 // multiplier per hit

// Rules
WIN_SCORE: 11
```

## Network Protocol

Binary WebSocket via postcard serialization.

**Client → Server:**
```rust
enum C2S {
    Join { code: [u8; 5] },
    Input { player_id: u8, paddle_dir: i8 },  // -1=up, 0=stop, 1=down
    Ping { t_ms: u32 },
}
```

**Server → Client:**
```rust
enum S2C {
    Welcome { player_id: u8 },
    GameState { tick, ball_x, ball_y, ball_vx, ball_vy, 
                paddle_left_y, paddle_right_y, score_left, score_right },
    GameOver { winner: u8 },
    Pong { t_ms: u32 },
}
```

## ECS Components

```rust
Paddle { player_id: u8, y: f32 }
Ball { pos: Vec2, vel: Vec2 }
PaddleIntent { dir: i8 }
```

**System order:** IngestInputs → MovePaddles → MoveBall → CheckCollisions → CheckScoring → ResetBall

## Timing

| What | Rate | Why |
|------|------|-----|
| Server simulation | 60 Hz | Physics accuracy |
| State broadcast | 20 Hz | Cost optimization (66% reduction) |
| Client rendering | 120 Hz target | Smooth visuals |

## Physics

- **Wall collision:** Reflect Y velocity
- **Paddle collision:** Reflect X velocity + apply spin
  - Hit position affects trajectory (top→up, bottom→down)
  - Moving paddle adds 30% of its velocity to ball
  - Max deflection: ~45°
  - Speed multiplied by 1.05x (capped at 16 u/s)

## Client Prediction

Own paddle uses predicted state for instant response. Server state provides authoritative correction. Opponent and ball interpolated from server snapshots.

## Durable Object Lifecycle

1. `/create` → Generate 5-char code, spawn DO
2. WebSocket connect → Assign player_id (0 or 1)
3. 60 Hz alarm → Run simulation, broadcast every 3rd tick
4. Score reaches 11 → Send GameOver, stop alarms
5. No clients → 1 minute idle timeout

---

*Source: [game_core/src/params.rs](game_core/src/params.rs)*
