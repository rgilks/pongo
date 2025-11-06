use hecs::World;

use crate::components::*;
use crate::resources::*;
use crate::params::Params;

/// Advance spawn pads and spawn pickup items
pub fn pickups_spawn(world: &mut World, time: &Time, map: &GameMap, rng: &mut crate::resources::GameRng) {
    // Find or create spawn pads
    let mut pads = Vec::new();
    
    for (entity, pad) in world.query::<&SpawnPad>().iter() {
        pads.push((entity, pad.clone()));
    }
    pads.sort_by_key(|(e, _)| e.id());

    // If no pads exist, create them from map
    if pads.is_empty() {
        for (pos, kind) in &map.map.pickup_pads {
            let respawn_min = Params::PICKUP_RESPAWN_MIN;
            let respawn_max = Params::PICKUP_RESPAWN_MAX;
            let pad = SpawnPad::new(*kind, respawn_min, respawn_max);
            let transform = Transform2D::new(*pos, 0.0);
            world.spawn((pad, transform));
        }
        return;
    }

    // Update pads and spawn items
    for (entity, mut pad) in pads {
        pad.t_until -= time.dt;

        if pad.t_until <= 0.0 {
            // Check if item already exists at this pad
            let pad_transform = *world.get::<&Transform2D>(entity).unwrap();
            let mut has_item = false;
            
            for (_, (_, transform)) in world.query::<(&Pickup, &Transform2D)>().iter() {
                if (transform.pos - pad_transform.pos).length() < 0.5 {
                    has_item = true;
                    break;
                }
            }

            if !has_item {
                // Spawn pickup
                let pickup = Pickup { kind: pad.kind };
                let transform = Transform2D::new(pad_transform.pos, 0.0);
                world.spawn((pickup, transform));

                // Reset respawn timer
                pad.t_until = rng.gen_range(pad.respawn_min, pad.respawn_max);
            } else {
                // Item exists, reset timer
                pad.t_until = rng.gen_range(pad.respawn_min, pad.respawn_max);
            }
        }

        // Update pad
        for (e, mut p) in world.query_mut::<&mut SpawnPad>() {
            if e == entity {
                *p = pad;
                break;
            }
        }
    }
}

/// Collect pickups when players touch them
pub fn pickups_collect(world: &mut World, events: &mut Events) {
    // Collect pickups (deterministic: sort by entity ID)
    let mut pickup_entities: Vec<_> = world.query::<(&Pickup, &Transform2D)>()
        .iter()
        .map(|(e, _)| e)
        .collect();
    pickup_entities.sort_by_key(|e| e.id());

    // Collect players (deterministic: sort by entity ID)
    let mut player_entities: Vec<_> = world.query::<(&Player, &Transform2D)>()
        .iter()
        .map(|(e, _)| e)
        .collect();
    player_entities.sort_by_key(|e| e.id());

    let mut pickups_to_remove = Vec::new();

    for pickup_entity in &pickup_entities {
        let pickup = *world.get::<&Pickup>(*pickup_entity).unwrap();
        let pickup_transform = *world.get::<&Transform2D>(*pickup_entity).unwrap();

        for player_entity in &player_entities {
            let player = *world.get::<&Player>(*player_entity).unwrap();
            let player_transform = *world.get::<&Transform2D>(*player_entity).unwrap();

            // Check collision
            let dist = (pickup_transform.pos - player_transform.pos).length();
            let collision_dist = Params::PLAYER_RADIUS + 0.3; // pickup radius ~0.3

            if dist < collision_dist {
                events.pickup_taken.push(crate::resources::PickupTakenEvent {
                    player_id: player.id,
                    pickup_entity: *pickup_entity,
                    kind: pickup.kind,
                });
                pickups_to_remove.push(*pickup_entity);
                break;
            }
        }
    }

    // Remove collected pickups
    for entity in pickups_to_remove {
        let _ = world.despawn(entity);
    }
}

/// Apply pickup effects
pub fn apply_pickup_effects(world: &mut World, events: &Events) {
    for event in &events.pickup_taken {
        // Find player entity
        let mut player_entity = None;
        for (entity, player) in world.query::<&Player>().iter() {
            if player.id == event.player_id {
                player_entity = Some(entity);
                break;
            }
        }

        if let Some(entity) = player_entity {
            match event.kind {
                PickupKind::Health => {
                    let health = *world.get::<&Health>(entity).unwrap();
                    let mut health = health;
                    if health.damage > 0 {
                        health.damage -= 1;
                    }
                    for (e, mut h) in world.query_mut::<&mut Health>() {
                        if e == entity {
                            *h = health;
                            break;
                        }
                    }
                }
                PickupKind::BoltUpgrade => {
                    // Check if component exists first
                    let has_bolt_max = world.get::<&crate::components::BoltMaxLevel>(entity).is_ok();
                    if has_bolt_max {
                        // Update existing
                        let mut found = false;
                        for (e, mut b) in world.query_mut::<&mut crate::components::BoltMaxLevel>() {
                            if e == entity {
                                if b.level < 3 {
                                    b.level += 1;
                                }
                                found = true;
                                break;
                            }
                        }
                        if !found {
                            // Shouldn't happen, but create if missing
                            let mut bolt_max = crate::components::BoltMaxLevel::new();
                            bolt_max.level = 2;
                            world.insert(entity, (bolt_max,)).unwrap();
                        }
                    } else {
                        // Create if doesn't exist
                        let mut bolt_max = crate::components::BoltMaxLevel::new();
                        bolt_max.level = 2; // Upgrade to L2
                        world.insert(entity, (bolt_max,)).unwrap();
                    }
                }
                PickupKind::ShieldModule => {
                    let shield = *world.get::<&Shield>(entity).unwrap();
                    let mut shield = shield;
                    if shield.max < 3 {
                        shield.max += 1;
                    }
                    for (e, mut s) in world.query_mut::<&mut Shield>() {
                        if e == entity {
                            *s = shield;
                            break;
                        }
                    }
                }
            }
        }
    }
}
