use crate::{Ball, Config, Events, GameMap, Paddle, PaddleIntent};
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

    // Collect paddle data with intents (for velocity calculation)
    let paddles: Vec<(u8, f32, i8)> = world
        .query::<(&Paddle, &PaddleIntent)>()
        .iter()
        .map(|(_e, (p, intent))| (p.player_id, p.y, intent.dir))
        .collect();

    // Check paddle collisions
    let ball_radius = config.ball_radius;

    for (player_id, paddle_y, paddle_dir) in paddles {
        let paddle_x = config.paddle_x(player_id);
        let paddle_half_width = config.paddle_width / 2.0;
        let paddle_half_height = config.paddle_height / 2.0;

        // Simple AABB collision check
        let dx = (ball_pos.x - paddle_x).abs();
        let dy = (ball_pos.y - paddle_y).abs();

        if dx < paddle_half_width + ball_radius && dy < paddle_half_height + ball_radius {
            // Collision detected! Check if ball is moving toward paddle
            let should_bounce =
                (player_id == 0 && ball_vel.x < 0.0) || (player_id == 1 && ball_vel.x > 0.0);

            if should_bounce {
                // Calculate where on the paddle the ball hit
                // Relative position from -1 (top) to 1 (bottom)
                let hit_relative_y = (ball_pos.y - paddle_y) / paddle_half_height;
                // Clamp to [-1, 1] to handle edge cases
                let hit_relative_y = hit_relative_y.clamp(-1.0, 1.0);

                // Calculate paddle velocity (positive = down, negative = up)
                let paddle_velocity = paddle_dir as f32 * config.paddle_speed;

                // Calculate new ball velocity
                // Base X velocity: reflect horizontally and increase speed
                let base_speed = ball_vel.length();
                let new_speed =
                    (base_speed * config.ball_speed_increase).min(config.ball_speed_max);

                // Y velocity is affected by:
                // 1. Hit position on paddle (top = negative Y, bottom = positive Y)
                // 2. Paddle movement (add paddle velocity component)
                // Maximum deflection angle (in radians) - controls how much paddle position affects trajectory
                let max_deflection_angle = 0.785; // ~45 degrees
                let y_deflection = hit_relative_y * max_deflection_angle * new_speed;

                // Add paddle velocity influence (30% of paddle speed affects ball)
                let paddle_influence = paddle_velocity * 0.3;

                // Calculate new velocity direction
                // X: always away from paddle (positive for left paddle, negative for right)
                let new_vx = if player_id == 0 {
                    new_speed.abs() // Right
                } else {
                    -new_speed.abs() // Left
                };

                // Y: combine deflection from hit position and paddle movement
                let new_vy = y_deflection + paddle_influence;

                // Normalize to maintain consistent speed
                let new_vel = glam::Vec2::new(new_vx, new_vy);
                let normalized_vel = new_vel.normalize() * new_speed;

                ball_vel = normalized_vel;

                // Push ball out of paddle
                if player_id == 0 {
                    ball_pos.x = paddle_x + paddle_half_width + ball_radius;
                } else {
                    ball_pos.x = paddle_x - paddle_half_width - ball_radius;
                }

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
