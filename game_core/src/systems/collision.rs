use crate::{Ball, Config, Events, GameMap, Paddle};
use hecs::World;

/// Check ball collisions with walls and paddles
pub fn check_collisions(world: &mut World, map: &GameMap, config: &Config, events: &mut Events) {
    // First, collect ball and paddle data without holding borrows
    let ball_data = {
        let mut ball_query = world.query::<&Ball>();
        ball_query
            .iter()
            .next()
            .map(|(_e, ball)| (ball.pos, ball.vel))
    };

    let (mut ball_pos, mut ball_vel) = match ball_data {
        Some(data) => data,
        None => return, // No ball in world
    };

    // Check top/bottom wall bounces
    let half_height = config.ball_radius;
    if ball_pos.y - half_height <= 0.0 || ball_pos.y + half_height >= map.height {
        ball_vel.y = -ball_vel.y;
        // Clamp position to prevent stuck
        if ball_pos.y - half_height <= 0.0 {
            ball_pos.y = half_height;
        }
        if ball_pos.y + half_height >= map.height {
            ball_pos.y = map.height - half_height;
        }
        events.ball_hit_wall = true;

        // Update ball
        for (_entity, ball) in world.query_mut::<&mut Ball>() {
            ball.pos = ball_pos;
            ball.vel = ball_vel;
        }
    }

    // Collect paddle data
    let paddles: Vec<(u8, f32)> = world
        .query::<&Paddle>()
        .iter()
        .map(|(_e, p)| (p.player_id, p.y))
        .collect();

    // Check paddle collisions
    let ball_radius = config.ball_radius;

    for (player_id, paddle_y) in paddles {
        let paddle_x = config.paddle_x(player_id);
        let paddle_half_width = config.paddle_width / 2.0;
        let paddle_half_height = config.paddle_height / 2.0;

        // Simple AABB collision check
        let dx = (ball_pos.x - paddle_x).abs();
        let dy = (ball_pos.y - paddle_y).abs();

        if dx < paddle_half_width + ball_radius && dy < paddle_half_height + ball_radius {
            // Collision detected! Check if ball is moving toward paddle
            if player_id == 0 && ball_vel.x < 0.0 {
                // Left paddle, ball moving left
                ball_vel.x = ball_vel.x.abs(); // Reflect to right
                ball_pos.x = paddle_x + paddle_half_width + ball_radius; // Push out

                // Increase speed slightly
                let speed = ball_vel.length();
                let new_speed = (speed * config.ball_speed_increase).min(config.ball_speed_max);
                ball_vel = ball_vel.normalize() * new_speed;

                events.ball_hit_paddle = true;

                // Update ball
                for (_entity, ball) in world.query_mut::<&mut Ball>() {
                    ball.pos = ball_pos;
                    ball.vel = ball_vel;
                }
                return;
            } else if player_id == 1 && ball_vel.x > 0.0 {
                // Right paddle, ball moving right
                ball_vel.x = -ball_vel.x.abs(); // Reflect to left
                ball_pos.x = paddle_x - paddle_half_width - ball_radius; // Push out

                // Increase speed slightly
                let speed = ball_vel.length();
                let new_speed = (speed * config.ball_speed_increase).min(config.ball_speed_max);
                ball_vel = ball_vel.normalize() * new_speed;

                events.ball_hit_paddle = true;

                // Update ball
                for (_entity, ball) in world.query_mut::<&mut Ball>() {
                    ball.pos = ball_pos;
                    ball.vel = ball_vel;
                }
                return;
            }
        }
    }
}
