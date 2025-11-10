pub mod components;
pub mod map;
pub mod params;
pub mod resources;
pub mod systems;

pub use components::*;
pub use map::*;
pub use params::*;
pub use resources::*;

use hecs::World;
use systems::*;

/// Run the deterministic Pong game simulation
#[allow(clippy::too_many_arguments)]
pub fn step(
    world: &mut World,
    time: &mut Time,
    map: &GameMap,
    config: &Config,
    score: &mut Score,
    events: &mut Events,
    net_queue: &mut NetQueue,
    rng: &mut GameRng,
) {
    // Clamp dt to prevent large jumps
    let clamped_dt = time.dt.min(Params::MAX_DT);

    // Fixed micro-steps for stable physics
    let mut remaining_dt = clamped_dt;
    while remaining_dt > 0.0 {
        let step_dt = remaining_dt.min(Params::FIXED_DT);
        remaining_dt -= step_dt;

        let step_time = Time {
            dt: step_dt,
            now: time.now + (clamped_dt - remaining_dt),
        };

        // Clear events at start of frame
        events.clear();

        // 1. Ingest inputs (apply to paddle intents)
        ingest_inputs(world, net_queue);

        // 2. Move paddles based on intents
        move_paddles(world, &step_time, map, config);

        // 3. Move ball
        move_ball(world, &step_time);

        // 4. Check collisions (ball vs paddles, walls)
        check_collisions(world, map, config, events);

        // 5. Check scoring (ball exited arena)
        check_scoring(world, map, score, events, rng, config);
    }

    // Update time
    time.now += clamped_dt;
}

/// Helper to create a paddle entity
pub fn create_paddle(world: &mut World, player_id: u8, y: f32) -> hecs::Entity {
    world.spawn((Paddle::new(player_id, y), PaddleIntent::new()))
}

/// Helper to create the ball entity
pub fn create_ball(world: &mut World, pos: glam::Vec2, vel: glam::Vec2) -> hecs::Entity {
    world.spawn((Ball::new(pos, vel),))
}
