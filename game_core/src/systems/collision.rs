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

        if dx < paddle_half_width + ball_radius - config.ball_paddle_overlap
            && dy < paddle_half_height + ball_radius
        {
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

                // Push ball out of paddle (respecting overlap)
                if player_id == 0 {
                    ball_pos.x =
                        paddle_x + paddle_half_width + ball_radius - config.ball_paddle_overlap;
                } else {
                    ball_pos.x =
                        paddle_x - paddle_half_width - ball_radius + config.ball_paddle_overlap;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{create_ball, create_paddle, Ball, Config, Events, GameMap};

    fn setup_world() -> (hecs::World, Config, GameMap, Events) {
        let world = hecs::World::new();
        let config = Config::new();
        let map = GameMap::new();
        let events = Events::new();
        (world, config, map, events)
    }

    #[test]
    fn test_ball_bounces_off_top_wall() {
        let (mut world, config, map, mut events) = setup_world();
        let ball_pos = glam::Vec2::new(16.0, config.ball_radius - 0.1); // Above top wall
        let ball_vel = glam::Vec2::new(8.0, -4.0); // Moving up
        create_ball(&mut world, ball_pos, ball_vel);

        check_collisions(&mut world, &map, &config, &mut events);

        // Verify ball bounced (Y velocity reversed)
        for (_entity, ball) in world.query::<&Ball>().iter() {
            assert!(
                ball.vel.y > 0.0,
                "Ball should bounce down after hitting top wall"
            );
            assert_eq!(ball.vel.x, ball_vel.x, "X velocity should be unchanged");
            assert!(
                ball.pos.y >= config.ball_radius,
                "Ball should be pushed out of wall"
            );
        }
        assert!(events.ball_hit_wall, "Should trigger ball_hit_wall event");
    }

    #[test]
    fn test_ball_bounces_off_bottom_wall() {
        let (mut world, config, map, mut events) = setup_world();
        let ball_pos = glam::Vec2::new(16.0, map.height - config.ball_radius + 0.1); // Below bottom wall
        let ball_vel = glam::Vec2::new(8.0, 4.0); // Moving down
        create_ball(&mut world, ball_pos, ball_vel);

        check_collisions(&mut world, &map, &config, &mut events);

        // Verify ball bounced (Y velocity reversed)
        for (_entity, ball) in world.query::<&Ball>().iter() {
            assert!(
                ball.vel.y < 0.0,
                "Ball should bounce up after hitting bottom wall"
            );
            assert_eq!(ball.vel.x, ball_vel.x, "X velocity should be unchanged");
            assert!(
                ball.pos.y <= map.height - config.ball_radius,
                "Ball should be pushed out of wall"
            );
        }
        assert!(events.ball_hit_wall, "Should trigger ball_hit_wall event");
    }

    #[test]
    fn test_ball_collides_with_left_paddle() {
        let (mut world, config, map, mut events) = setup_world();
        let paddle_x = config.paddle_x(0);
        let paddle_y = 12.0;
        create_paddle(&mut world, 0, paddle_y);

        // Position ball to hit paddle (inside collision bounds)
        let paddle_half_width = config.paddle_width / 2.0;
        let ball_pos = glam::Vec2::new(
            paddle_x + paddle_half_width - config.ball_radius * 0.5,
            paddle_y,
        );
        let ball_vel = glam::Vec2::new(-8.0, 0.0); // Moving left toward paddle
        create_ball(&mut world, ball_pos, ball_vel);

        check_collisions(&mut world, &map, &config, &mut events);

        // Verify ball bounced (X velocity reversed)
        for (_entity, ball) in world.query::<&Ball>().iter() {
            assert!(
                ball.vel.x > 0.0,
                "Ball should bounce right after hitting left paddle"
            );
            assert!(ball.pos.x > paddle_x, "Ball should be pushed out of paddle");
        }
        assert!(
            events.ball_hit_paddle,
            "Should trigger ball_hit_paddle event"
        );
    }

    #[test]
    fn test_ball_collides_with_right_paddle() {
        let (mut world, config, map, mut events) = setup_world();
        let paddle_x = config.paddle_x(1);
        let paddle_y = 12.0;
        create_paddle(&mut world, 1, paddle_y);

        // Position ball to hit paddle (inside collision bounds)
        let paddle_half_width = config.paddle_width / 2.0;
        let ball_pos = glam::Vec2::new(
            paddle_x - paddle_half_width + config.ball_radius * 0.5,
            paddle_y,
        );
        let ball_vel = glam::Vec2::new(8.0, 0.0); // Moving right toward paddle
        create_ball(&mut world, ball_pos, ball_vel);

        check_collisions(&mut world, &map, &config, &mut events);

        // Verify ball bounced (X velocity reversed)
        for (_entity, ball) in world.query::<&Ball>().iter() {
            assert!(
                ball.vel.x < 0.0,
                "Ball should bounce left after hitting right paddle"
            );
            assert!(ball.pos.x < paddle_x, "Ball should be pushed out of paddle");
        }
        assert!(
            events.ball_hit_paddle,
            "Should trigger ball_hit_paddle event"
        );
    }

    #[test]
    fn test_ball_speed_increases_on_paddle_hit() {
        let (mut world, config, map, mut events) = setup_world();
        let paddle_x = config.paddle_x(0);
        let paddle_y = 12.0;
        create_paddle(&mut world, 0, paddle_y);

        let initial_speed = 8.0;
        let paddle_half_width = config.paddle_width / 2.0;
        let ball_pos = glam::Vec2::new(
            paddle_x + paddle_half_width - config.ball_radius * 0.5,
            paddle_y,
        );
        let ball_vel = glam::Vec2::new(-initial_speed, 0.0);
        create_ball(&mut world, ball_pos, ball_vel);

        check_collisions(&mut world, &map, &config, &mut events);

        // Verify ball speed increased
        for (_entity, ball) in world.query::<&Ball>().iter() {
            let new_speed = ball.vel.length();
            let expected_speed =
                (initial_speed * config.ball_speed_increase).min(config.ball_speed_max);
            assert!(
                (new_speed - expected_speed).abs() < 0.01,
                "Ball speed should increase by {}x, got {}",
                config.ball_speed_increase,
                new_speed
            );
        }
    }

    #[test]
    fn test_ball_speed_caps_at_max() {
        let (mut world, config, map, mut events) = setup_world();
        let paddle_x = config.paddle_x(0);
        let paddle_y = 12.0;
        create_paddle(&mut world, 0, paddle_y);

        // Start with speed near max
        let initial_speed = config.ball_speed_max - 1.0;
        let ball_pos = glam::Vec2::new(
            paddle_x + config.paddle_width / 2.0 + config.ball_radius,
            paddle_y,
        );
        let ball_vel = glam::Vec2::new(-initial_speed, 0.0);
        create_ball(&mut world, ball_pos, ball_vel);

        check_collisions(&mut world, &map, &config, &mut events);

        // Verify ball speed doesn't exceed max
        for (_entity, ball) in world.query::<&Ball>().iter() {
            let new_speed = ball.vel.length();
            assert!(
                new_speed <= config.ball_speed_max,
                "Ball speed should not exceed max {}",
                config.ball_speed_max
            );
        }
    }

    #[test]
    fn test_ball_trajectory_affected_by_hit_position() {
        let (mut world, config, map, mut events) = setup_world();
        let paddle_x = config.paddle_x(0);
        let paddle_y = 12.0;
        create_paddle(&mut world, 0, paddle_y);

        // Hit top of paddle (position ball inside collision bounds)
        let paddle_half_width = config.paddle_width / 2.0;
        let paddle_half_height = config.paddle_height / 2.0;
        let ball_pos_top = glam::Vec2::new(
            paddle_x + paddle_half_width - config.ball_radius * 0.5,
            paddle_y - paddle_half_height + 0.1,
        );
        let ball_vel = glam::Vec2::new(-8.0, 0.0);
        create_ball(&mut world, ball_pos_top, ball_vel);

        check_collisions(&mut world, &map, &config, &mut events);

        // Verify ball deflects upward
        for (_entity, ball) in world.query::<&Ball>().iter() {
            assert!(
                ball.vel.y < 0.0,
                "Ball should deflect upward when hitting top of paddle"
            );
        }

        // Reset and test bottom hit
        world.clear();
        events.clear();
        create_paddle(&mut world, 0, paddle_y);

        let ball_pos_bottom = glam::Vec2::new(
            paddle_x + paddle_half_width - config.ball_radius * 0.5,
            paddle_y + paddle_half_height - 0.1,
        );
        create_ball(&mut world, ball_pos_bottom, ball_vel);

        check_collisions(&mut world, &map, &config, &mut events);

        // Verify ball deflects downward
        for (_entity, ball) in world.query::<&Ball>().iter() {
            assert!(
                ball.vel.y > 0.0,
                "Ball should deflect downward when hitting bottom of paddle"
            );
        }
    }

    #[test]
    fn test_ball_does_not_bounce_when_moving_away_from_paddle() {
        let (mut world, config, map, mut events) = setup_world();
        let paddle_x = config.paddle_x(0);
        let paddle_y = 12.0;
        create_paddle(&mut world, 0, paddle_y);

        // Ball is at paddle but moving away (right)
        let ball_pos = glam::Vec2::new(
            paddle_x + config.paddle_width / 2.0 + config.ball_radius,
            paddle_y,
        );
        let ball_vel = glam::Vec2::new(8.0, 0.0); // Moving right (away from left paddle)
        create_ball(&mut world, ball_pos, ball_vel);

        check_collisions(&mut world, &map, &config, &mut events);

        // Verify ball didn't bounce
        for (_entity, ball) in world.query::<&Ball>().iter() {
            assert_eq!(
                ball.vel.x, ball_vel.x,
                "Ball should not bounce when moving away"
            );
        }
        assert!(
            !events.ball_hit_paddle,
            "Should not trigger collision when moving away"
        );
    }

    #[test]
    fn test_no_collision_when_no_ball() {
        let (mut world, config, map, mut events) = setup_world();
        create_paddle(&mut world, 0, 12.0);

        // Should not panic or error
        check_collisions(&mut world, &map, &config, &mut events);

        assert!(!events.ball_hit_paddle);
        assert!(!events.ball_hit_wall);
    }

    #[test]
    fn test_ball_paddle_overlap() {
        let (mut world, config, map, mut events) = setup_world();
        let paddle_x = config.paddle_x(0);
        let paddle_y = 12.0;
        create_paddle(&mut world, 0, paddle_y);

        let paddle_half_width = config.paddle_width / 2.0;
        let ball_radius = config.ball_radius;
        let overlap = config.ball_paddle_overlap;

        // Position ball such that it's just outside the overlap threshold
        let start_x = paddle_x + paddle_half_width + ball_radius - overlap + 0.01;
        let ball_pos = glam::Vec2::new(start_x, paddle_y);
        let ball_vel = glam::Vec2::new(-8.0, 0.0);
        create_ball(&mut world, ball_pos, ball_vel);

        // First check: no collision yet
        check_collisions(&mut world, &map, &config, &mut events);
        assert!(!events.ball_hit_paddle);

        // Move ball slightly inside the threshold
        for (_e, ball) in world.query_mut::<&mut Ball>() {
            ball.pos.x -= 0.02;
        }

        // Second check: collision should trigger
        check_collisions(&mut world, &map, &config, &mut events);
        assert!(events.ball_hit_paddle);

        // Verify push-out position respects overlap
        for (_e, ball) in world.query::<&Ball>().iter() {
            let expected_x = paddle_x + paddle_half_width + ball_radius - overlap;
            assert!(
                (ball.pos.x - expected_x).abs() < 0.001,
                "Ball should be pushed out to the overlap point, got {}, expected {}",
                ball.pos.x,
                expected_x
            );
        }
    }
}
