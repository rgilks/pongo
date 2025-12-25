use crate::{NetQueue, Paddle};
use hecs::World;

/// Ingest network inputs and apply to paddle positions
pub fn ingest_inputs(world: &mut World, net_queue: &mut NetQueue) {
    // Process all queued inputs
    for (player_id, y_pos) in net_queue.inputs.drain(..) {
        // Find paddle with matching player_id
        for (_entity, paddle) in world.query_mut::<&mut Paddle>() {
            if paddle.player_id == player_id {
                // Apply absolute position (clamped to arena)
                // Arena height is 24.0, paddle height 4.0.
                // Valid range: 2.0 to 22.0 (center pos)
                // Wait, previous code used Clamp(2.0, 22.0)?
                // Let's check previous CLAMP values used in client:
                // client.local_paddle_y.clamp(half_height, ARENA_HEIGHT - half_height);
                // half_height = 2.0. Arena = 24.0. So 2.0 to 22.0 is center position range.
                paddle.y = y_pos.clamp(2.0, 22.0);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{create_paddle, NetQueue, Paddle};

    fn setup_world() -> (hecs::World, NetQueue) {
        (hecs::World::new(), NetQueue::new())
    }

    #[test]
    fn test_input_applied_to_correct_paddle() {
        let (mut world, mut net_queue) = setup_world();
        create_paddle(&mut world, 0, 12.0);
        create_paddle(&mut world, 1, 12.0);

        // Queue input for player 0
        net_queue.push_input(0, 5.0);
        net_queue.push_input(1, 18.0);

        ingest_inputs(&mut world, &mut net_queue);

        // Verify positions were applied correctly
        let mut paddle_y = Vec::new();
        for (_entity, paddle) in world.query::<&Paddle>().iter() {
            paddle_y.push((paddle.player_id, paddle.y));
        }
        paddle_y.sort_by_key(|(id, _)| *id);

        assert_eq!(paddle_y.len(), 2);
        assert_eq!(paddle_y[0], (0, 5.0));
        assert_eq!(paddle_y[1], (1, 18.0));
    }

    #[test]
    fn test_input_queue_cleared_after_processing() {
        let (mut world, mut net_queue) = setup_world();
        create_paddle(&mut world, 0, 12.0);

        net_queue.push_input(0, 10.0);
        assert_eq!(net_queue.inputs.len(), 1);

        ingest_inputs(&mut world, &mut net_queue);

        assert_eq!(net_queue.inputs.len(), 0, "Input queue should be cleared");
    }

    #[test]
    fn test_multiple_inputs_for_same_player() {
        let (mut world, mut net_queue) = setup_world();
        create_paddle(&mut world, 0, 12.0);

        // Queue multiple inputs (last one should win)
        net_queue.push_input(0, 5.0);
        net_queue.push_input(0, 15.0);
        net_queue.push_input(0, 8.0); // Last

        ingest_inputs(&mut world, &mut net_queue);

        // Last input should be applied
        for (_entity, paddle) in world.query::<&Paddle>().iter() {
            assert_eq!(paddle.y, 8.0, "Last input should be applied");
        }
    }

    #[test]
    fn test_clamping() {
        let (mut world, mut net_queue) = setup_world();
        create_paddle(&mut world, 0, 12.0);

        net_queue.push_input(0, -100.0); // Too low
        ingest_inputs(&mut world, &mut net_queue);
        for (_entity, paddle) in world.query::<&Paddle>().iter() {
            assert_eq!(paddle.y, 2.0, "Should clamp to min");
        }

        net_queue.push_input(0, 100.0); // Too high
        ingest_inputs(&mut world, &mut net_queue);
        for (_entity, paddle) in world.query::<&Paddle>().iter() {
            assert_eq!(paddle.y, 22.0, "Should clamp to max");
        }
    }

    #[test]
    fn test_no_panic_when_no_paddles() {
        let (mut world, mut net_queue) = setup_world();
        net_queue.push_input(0, 10.0);

        // Should not panic
        ingest_inputs(&mut world, &mut net_queue);
    }

    #[test]
    fn test_no_panic_when_no_inputs() {
        let (mut world, mut net_queue) = setup_world();
        create_paddle(&mut world, 0, 12.0);

        // Should not panic
        ingest_inputs(&mut world, &mut net_queue);
    }
}
