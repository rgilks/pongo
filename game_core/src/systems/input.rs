use crate::{NetQueue, Paddle, PaddleIntent};
use hecs::World;

/// Ingest network inputs and apply to paddle intents
pub fn ingest_inputs(world: &mut World, net_queue: &mut NetQueue) {
    // Process all queued inputs
    for (player_id, dir) in net_queue.inputs.drain(..) {
        // Find paddle with matching player_id
        let mut found = false;
        for (_entity, (paddle, intent)) in world.query_mut::<(&Paddle, &mut PaddleIntent)>() {
            if paddle.player_id == player_id {
                intent.dir = dir;
                found = true;
            }
        }
        if !found {
            #[cfg(target_arch = "wasm32")]
            web_sys::console::log_1(
                &format!("⚠️  No paddle found for player_id={}", player_id).into(),
            );
        }
    }
}
