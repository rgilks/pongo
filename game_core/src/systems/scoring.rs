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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{create_ball, Ball, Config, Events, GameMap, GameRng, Score};

    fn setup_world() -> (hecs::World, Config, GameMap, Score, Events, GameRng) {
        let world = hecs::World::new();
        let config = Config::new();
        let map = GameMap::new();
        let score = Score::new();
        let events = Events::new();
        let rng = GameRng::new(12345); // Fixed seed for deterministic tests
        (world, config, map, score, events, rng)
    }

    #[test]
    fn test_right_player_scores_when_ball_exits_left() {
        let (mut world, config, map, mut score, mut events, mut rng) = setup_world();
        let ball_pos = glam::Vec2::new(-0.1, 12.0); // Left of arena
        let ball_vel = glam::Vec2::new(-8.0, 0.0);
        create_ball(&mut world, ball_pos, ball_vel);

        check_scoring(&mut world, &map, &mut score, &mut events, &mut rng, &config);

        assert_eq!(score.right, 1, "Right player should score");
        assert_eq!(score.left, 0, "Left player should not score");
        assert!(events.right_scored, "Should trigger right_scored event");
    }

    #[test]
    fn test_left_player_scores_when_ball_exits_right() {
        let (mut world, config, map, mut score, mut events, mut rng) = setup_world();
        let ball_pos = glam::Vec2::new(map.width + 0.1, 12.0); // Right of arena
        let ball_vel = glam::Vec2::new(8.0, 0.0);
        create_ball(&mut world, ball_pos, ball_vel);

        check_scoring(&mut world, &map, &mut score, &mut events, &mut rng, &config);

        assert_eq!(score.left, 1, "Left player should score");
        assert_eq!(score.right, 0, "Right player should not score");
        assert!(events.left_scored, "Should trigger left_scored event");
    }

    #[test]
    fn test_ball_resets_after_scoring() {
        let (mut world, config, map, mut score, mut events, mut rng) = setup_world();
        let ball_pos = glam::Vec2::new(-0.1, 12.0);
        let ball_vel = glam::Vec2::new(-8.0, 0.0);
        create_ball(&mut world, ball_pos, ball_vel);

        check_scoring(&mut world, &map, &mut score, &mut events, &mut rng, &config);

        // Verify ball was reset to center
        for (_entity, ball) in world.query::<&Ball>().iter() {
            let center = map.ball_spawn();
            assert!(
                (ball.pos.x - center.x).abs() < 0.1 && (ball.pos.y - center.y).abs() < 0.1,
                "Ball should reset to center after scoring"
            );
            // Ball should have new velocity (reset with random direction)
            assert!(
                ball.vel.length() > 0.0,
                "Ball should have velocity after reset"
            );
        }
    }

    #[test]
    fn test_no_scoring_when_ball_in_bounds() {
        let (mut world, config, map, mut score, mut events, mut rng) = setup_world();
        let ball_pos = glam::Vec2::new(16.0, 12.0); // Center of arena
        let ball_vel = glam::Vec2::new(8.0, 4.0);
        create_ball(&mut world, ball_pos, ball_vel);

        check_scoring(&mut world, &map, &mut score, &mut events, &mut rng, &config);

        assert_eq!(score.left, 0, "No score when ball in bounds");
        assert_eq!(score.right, 0, "No score when ball in bounds");
        assert!(
            !events.left_scored && !events.right_scored,
            "No scoring events"
        );
    }

    #[test]
    fn test_multiple_scores_accumulate() {
        let (mut world, config, map, mut score, mut events, mut rng) = setup_world();

        // Left player scores
        create_ball(
            &mut world,
            glam::Vec2::new(map.width + 0.1, 12.0),
            glam::Vec2::new(8.0, 0.0),
        );
        check_scoring(&mut world, &map, &mut score, &mut events, &mut rng, &config);
        events.clear();

        // Left player scores again
        create_ball(
            &mut world,
            glam::Vec2::new(map.width + 0.1, 12.0),
            glam::Vec2::new(8.0, 0.0),
        );
        check_scoring(&mut world, &map, &mut score, &mut events, &mut rng, &config);

        assert_eq!(score.left, 2, "Scores should accumulate");
        assert_eq!(score.right, 0);
    }
}
