use crate::{Ball, Config, GameMap, Paddle, PaddleIntent, Time};
use hecs::World;

/// Apply paddle movement based on intents
pub fn move_paddles(world: &mut World, time: &Time, map: &GameMap, config: &Config) {
    for (_entity, (paddle, intent)) in world.query_mut::<(&mut Paddle, &PaddleIntent)>() {
        if intent.dir != 0 {
            let delta = intent.dir as f32 * config.paddle_speed * time.dt;
            paddle.y += delta;

            // Clamp to arena bounds
            paddle.y = map.clamp_y(paddle.y, config.paddle_height / 2.0);
        }
    }
}

/// Move ball based on velocity
pub fn move_ball(world: &mut World, time: &Time) {
    for (_entity, ball) in world.query_mut::<&mut Ball>() {
        ball.pos += ball.vel * time.dt;
    }
}
