# Pongo — Technical Specification

## Architecture

| Layer | Technology | Description |
|-------|------------|-------------|
| Client | Rust → WASM, [wgpu](https://wgpu.rs/) v24 | WebGPU renderer, client prediction |
| Server | Rust [Durable Object](https://developers.cloudflare.com/durable-objects/) | Authoritative 60 Hz simulation |
| ECS | [hecs](https://docs.rs/hecs) | Shared game logic |
| Protocol | [postcard](https://docs.rs/postcard) | Binary WebSocket messages |

## Game Constants

| Constant | Value | Unit |
|----------|-------|------|
| Arena | 32 × 24 | units |
| Paddle | 0.8 × 4.0 | units |
| Paddle speed | 12 | units/sec |
| Ball radius | 0.5 | units |
| Ball speed | 8 → 16 | units/sec |
| Speed multiplier | 1.05× | per hit |
| Win score | 11 | points |

## Network Protocol

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
    GameState { tick, ball, paddles, score },
    GameOver { winner: u8 },
    Pong { t_ms: u32 },
}
```

## ECS

**Components:** `Paddle { player_id, y }` · `Ball { pos, vel }` · `PaddleIntent { dir }`

**Systems:** IngestInputs → MovePaddles → MoveBall → CheckCollisions → CheckScoring → ResetBall

## Timing

| What | Rate | Reason |
|------|------|--------|
| Server tick | 60 Hz | Physics accuracy |
| State broadcast | 20 Hz | Cost optimization |
| Client render | 120 Hz | Smooth visuals |

## Physics

- **Walls:** Reflect Y velocity
- **Paddles:** Reflect X velocity + spin based on hit position and paddle movement
- **Speed:** +5% per hit, max 16 u/s

## Client Prediction

Own paddle uses local prediction for instant response. Server provides authoritative correction.

---

*Constants defined in [game_core/src/params.rs](game_core/src/params.rs)*
