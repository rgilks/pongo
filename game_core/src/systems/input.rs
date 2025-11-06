use hecs::World;

use crate::components::*;
use crate::resources::*;

/// Ingest input events and create movement/combat intents
pub fn ingest_inputs(world: &mut World, net_queue: &mut NetQueue) {
    // Process input events
    for input in &net_queue.inputs {
        // Find player entity
        let mut player_entity = None;
        for (entity, player) in world.query::<&Player>().iter() {
            if player.id == input.player_id {
                player_entity = Some(entity);
                break;
            }
        }

        if let Some(entity) = player_entity {
            // Create movement intent
            let intent = MovementIntent {
                thrust: input.thrust,
                turn: input.turn,
            };
            world.insert(entity, (intent,)).unwrap();

            // Create combat intent
            let combat = CombatIntent {
                bolt_level: input.bolt_level,
                shield_level: input.shield_level,
            };
            world.insert(entity, (combat,)).unwrap();
        }
    }

    // Clear processed inputs
    net_queue.inputs.clear();
}

/// Apply shield intents
pub fn apply_shield_intents(world: &mut World) {
    let mut updates = Vec::new();

    // Collect all players with combat intents and shields (deterministic: sort by entity ID)
    for (entity, (intent, shield)) in world.query::<(&CombatIntent, &Shield)>().iter() {
        updates.push((entity, *intent, *shield));
    }
    updates.sort_by_key(|(e, _, _)| e.id());

    for (entity, intent, mut shield) in updates {
        if intent.shield_level > 0 && shield.can_activate(intent.shield_level) {
            shield.active = intent.shield_level;
            shield.t_left = crate::params::Params::SHIELD_MAX_DURATION;
        } else if intent.shield_level == 0 && shield.is_active() {
            // Deactivate shield
            shield.active = 0;
            shield.cooldown = crate::params::Params::SHIELD_COOLDOWN;
        }

        world.insert(entity, (shield,)).unwrap();
    }
}
