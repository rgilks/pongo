use crate::{NetQueue, Paddle, PaddleIntent};
use hecs::World;

/// Ingest network inputs and apply to paddle intents
pub fn ingest_inputs(world: &mut World, net_queue: &mut NetQueue) {
    // Process all queued inputs
    for (player_id, dir) in net_queue.inputs.drain(..) {
        // Find paddle with matching player_id
        for (_entity, (paddle, intent)) in world.query_mut::<(&Paddle, &mut PaddleIntent)>() {
            if paddle.player_id == player_id {
                intent.dir = dir;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{create_paddle, NetQueue, Paddle, PaddleIntent};

    fn setup_world() -> (hecs::World, NetQueue) {
        (hecs::World::new(), NetQueue::new())
    }

    #[test]
    fn test_input_applied_to_correct_paddle() {
        let (mut world, mut net_queue) = setup_world();
        create_paddle(&mut world, 0, 12.0);
        create_paddle(&mut world, 1, 12.0);

        // Queue input for player 0
        net_queue.push_input(0, -1); // Up
        net_queue.push_input(1, 1); // Down

        ingest_inputs(&mut world, &mut net_queue);

        // Verify intents were applied correctly
        let mut paddle_intents = Vec::new();
        for (_entity, (paddle, intent)) in world.query::<(&Paddle, &PaddleIntent)>().iter() {
            paddle_intents.push((paddle.player_id, intent.dir));
        }
        paddle_intents.sort_by_key(|(id, _)| *id);

        assert_eq!(paddle_intents.len(), 2);
        assert_eq!(paddle_intents[0], (0, -1), "Player 0 should have up intent");
        assert_eq!(
            paddle_intents[1],
            (1, 1),
            "Player 1 should have down intent"
        );
    }

    #[test]
    fn test_input_queue_cleared_after_processing() {
        let (mut world, mut net_queue) = setup_world();
        create_paddle(&mut world, 0, 12.0);

        net_queue.push_input(0, -1);
        assert_eq!(net_queue.inputs.len(), 1);

        ingest_inputs(&mut world, &mut net_queue);

        assert_eq!(net_queue.inputs.len(), 0, "Input queue should be cleared");
    }

    #[test]
    fn test_multiple_inputs_for_same_player() {
        let (mut world, mut net_queue) = setup_world();
        create_paddle(&mut world, 0, 12.0);

        // Queue multiple inputs (last one should win)
        net_queue.push_input(0, -1); // Up
        net_queue.push_input(0, 1); // Down
        net_queue.push_input(0, 0); // Stop

        ingest_inputs(&mut world, &mut net_queue);

        // Last input should be applied
        for (_entity, (_paddle, intent)) in world.query::<(&Paddle, &PaddleIntent)>().iter() {
            assert_eq!(intent.dir, 0, "Last input should be applied");
        }
    }

    #[test]
    fn test_no_panic_when_no_paddles() {
        let (mut world, mut net_queue) = setup_world();
        net_queue.push_input(0, -1);

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
