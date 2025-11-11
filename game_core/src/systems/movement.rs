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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{create_ball, create_paddle, Ball, Config, GameMap, Paddle, PaddleIntent, Time};

    fn setup_world() -> (World, Config, GameMap, Time) {
        let world = World::new();
        let config = Config::new();
        let map = GameMap::new();
        let time = Time::new(0.016, 0.0); // 60 Hz
        (world, config, map, time)
    }

    #[test]
    fn test_paddle_moves_up() {
        let (mut world, config, map, time) = setup_world();
        let paddle_y = 12.0;
        create_paddle(&mut world, 0, paddle_y);

        // Set intent to move up
        for (_entity, (_paddle, intent)) in world.query_mut::<(&Paddle, &mut PaddleIntent)>() {
            intent.dir = -1; // Up
        }

        move_paddles(&mut world, &time, &map, &config);

        // Verify paddle moved up
        for (_entity, paddle) in world.query::<&Paddle>().iter() {
            assert!(paddle.y < paddle_y, "Paddle should move up");
            assert_eq!(paddle.y, paddle_y - config.paddle_speed * time.dt);
        }
    }

    #[test]
    fn test_paddle_moves_down() {
        let (mut world, config, map, time) = setup_world();
        let paddle_y = 12.0;
        create_paddle(&mut world, 0, paddle_y);

        // Set intent to move down
        for (_entity, (_paddle, intent)) in world.query_mut::<(&Paddle, &mut PaddleIntent)>() {
            intent.dir = 1; // Down
        }

        move_paddles(&mut world, &time, &map, &config);

        // Verify paddle moved down
        for (_entity, paddle) in world.query::<&Paddle>().iter() {
            assert!(paddle.y > paddle_y, "Paddle should move down");
            assert_eq!(paddle.y, paddle_y + config.paddle_speed * time.dt);
        }
    }

    #[test]
    fn test_paddle_stops_when_intent_zero() {
        let (mut world, config, map, time) = setup_world();
        let paddle_y = 12.0;
        create_paddle(&mut world, 0, paddle_y);

        // Set intent to stop
        for (_entity, (_paddle, intent)) in world.query_mut::<(&Paddle, &mut PaddleIntent)>() {
            intent.dir = 0;
        }

        move_paddles(&mut world, &time, &map, &config);

        // Verify paddle didn't move
        for (_entity, paddle) in world.query::<&Paddle>().iter() {
            assert_eq!(paddle.y, paddle_y);
        }
    }

    #[test]
    fn test_paddle_clamps_to_top_boundary() {
        let (mut world, config, map, time) = setup_world();
        let half_height = config.paddle_height / 2.0;
        let paddle_y = half_height + 0.1; // Just above minimum
        create_paddle(&mut world, 0, paddle_y);

        // Try to move up beyond boundary
        for (_entity, (_paddle, intent)) in world.query_mut::<(&Paddle, &mut PaddleIntent)>() {
            intent.dir = -1;
        }

        move_paddles(&mut world, &time, &map, &config);

        // Verify paddle clamped to boundary
        for (_entity, paddle) in world.query::<&Paddle>().iter() {
            assert_eq!(
                paddle.y, half_height,
                "Paddle should be clamped to top boundary"
            );
        }
    }

    #[test]
    fn test_paddle_clamps_to_bottom_boundary() {
        let (mut world, config, map, time) = setup_world();
        let half_height = config.paddle_height / 2.0;
        let paddle_y = map.height - half_height - 0.1; // Just below maximum
        create_paddle(&mut world, 0, paddle_y);

        // Try to move down beyond boundary
        for (_entity, (_paddle, intent)) in world.query_mut::<(&Paddle, &mut PaddleIntent)>() {
            intent.dir = 1;
        }

        move_paddles(&mut world, &time, &map, &config);

        // Verify paddle clamped to boundary
        for (_entity, paddle) in world.query::<&Paddle>().iter() {
            assert_eq!(
                paddle.y,
                map.height - half_height,
                "Paddle should be clamped to bottom boundary"
            );
        }
    }

    #[test]
    fn test_ball_moves_with_velocity() {
        let (mut world, _config, _map, time) = setup_world();
        let initial_pos = glam::Vec2::new(16.0, 12.0);
        let velocity = glam::Vec2::new(8.0, 4.0);
        create_ball(&mut world, initial_pos, velocity);

        move_ball(&mut world, &time);

        // Verify ball moved
        for (_entity, ball) in world.query::<&Ball>().iter() {
            let expected_pos = initial_pos + velocity * time.dt;
            assert_eq!(ball.pos, expected_pos);
        }
    }

    #[test]
    fn test_ball_moves_with_zero_velocity() {
        let (mut world, _config, _map, time) = setup_world();
        let initial_pos = glam::Vec2::new(16.0, 12.0);
        let velocity = glam::Vec2::ZERO;
        create_ball(&mut world, initial_pos, velocity);

        move_ball(&mut world, &time);

        // Verify ball didn't move
        for (_entity, ball) in world.query::<&Ball>().iter() {
            assert_eq!(ball.pos, initial_pos);
        }
    }

    #[test]
    fn test_multiple_paddles_move_independently() {
        let (mut world, config, map, time) = setup_world();
        create_paddle(&mut world, 0, 12.0);
        create_paddle(&mut world, 1, 12.0);

        // Set different intents for each paddle
        for (_entity, (paddle, intent)) in world.query_mut::<(&Paddle, &mut PaddleIntent)>() {
            intent.dir = if paddle.player_id == 0 { -1 } else { 1 };
        }

        move_paddles(&mut world, &time, &map, &config);

        // Verify paddles moved in opposite directions
        let mut paddle_positions = Vec::new();
        for (_entity, paddle) in world.query::<&Paddle>().iter() {
            paddle_positions.push((paddle.player_id, paddle.y));
        }
        paddle_positions.sort_by_key(|(id, _)| *id);

        assert_eq!(paddle_positions.len(), 2);
        assert!(paddle_positions[0].1 < 12.0, "Left paddle should move up");
        assert!(
            paddle_positions[1].1 > 12.0,
            "Right paddle should move down"
        );
    }
}
