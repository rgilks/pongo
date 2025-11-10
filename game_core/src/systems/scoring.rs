use crate::{Ball, Config, Events, GameMap, GameRng, Score};
use hecs::World;

/// Check if ball left the arena (scoring)
pub fn check_scoring(
    world: &mut World,
    map: &GameMap,
    score: &mut Score,
    events: &mut Events,
    rng: &mut GameRng,
    config: &Config,
) {
    for (_entity, ball) in world.query_mut::<&mut Ball>() {
        // Check if ball exited left or right edge
        if ball.pos.x < 0.0 {
            // Right player scores
            score.increment_right();
            events.right_scored = true;

            // Reset ball
            ball.reset(config.ball_speed_initial, rng);
        } else if ball.pos.x > map.width {
            // Left player scores
            score.increment_left();
            events.left_scored = true;

            // Reset ball
            ball.reset(config.ball_speed_initial, rng);
        }
    }
}
