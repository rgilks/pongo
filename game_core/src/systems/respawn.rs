use hecs::World;

use crate::components::*;
use crate::resources::*;
use crate::params::Params;

/// Handle respawn timers and spawn players
pub fn respawn_tick(world: &mut World, time: &Time, map: &GameMap, events: &mut Events) {
    // Collect all respawn timers (deterministic: sort by entity ID)
    let mut entities: Vec<_> = world.query::<(&RespawnTimer, &Player)>()
        .iter()
        .map(|(e, _)| e)
        .collect();
    entities.sort_by_key(|e| e.id());

    for entity in entities {
        let timer = *world.get::<&RespawnTimer>(entity).unwrap();
        let player = *world.get::<&Player>(entity).unwrap();
        let mut timer = timer;

        timer.t_left -= time.dt;

        if timer.is_ready() {
            // Find safe spawn position
            let spawn_pos = map.map.spawns
                .get((player.id as usize) % map.map.spawns.len())
                .copied()
                .unwrap_or(crate::map::Map::test_map().spawns[0]);

            events.respawn.push(RespawnEvent {
                player_id: player.id,
                spawn_pos,
            });
        }
        
        // Update timer
        for (e, mut t) in world.query_mut::<&mut RespawnTimer>() {
            if e == entity {
                *t = timer;
                break;
            }
        }
    }
}

/// Apply respawn events
pub fn apply_respawns(world: &mut World, events: &Events) {
    for event in &events.respawn {
        // Find player entity
        let mut player_entity = None;
        for (entity, player) in world.query::<&Player>().iter() {
            if player.id == event.player_id {
                player_entity = Some(entity);
                break;
            }
        }

        if let Some(entity) = player_entity {
            // Reset health
            let mut health = Health::new();
            for (e, mut h) in world.query_mut::<&mut Health>() {
                if e == entity {
                    *h = health;
                    break;
                }
            }

            // Reset energy
            let mut energy = Energy::new();
            for (e, mut e_comp) in world.query_mut::<&mut Energy>() {
                if e == entity {
                    *e_comp = energy;
                    break;
                }
            }

            // Set position
            for (e, mut transform) in world.query_mut::<&mut Transform2D>() {
                if e == entity {
                    transform.pos = event.spawn_pos;
                    transform.yaw = 0.0;
                    break;
                }
            }

            // Reset velocity
            for (e, mut vel) in world.query_mut::<&mut Velocity2D>() {
                if e == entity {
                    vel.vel = glam::Vec2::ZERO;
                    break;
                }
            }

            // Give spawn shield
            let mut shield = world.get::<&Shield>(entity)
                .map(|s| *s)
                .unwrap_or_else(|_| Shield::new());
            shield.active = Params::RESPAWN_SHIELD_LEVEL;
            shield.t_left = Params::RESPAWN_SHIELD_DURATION;
            for (e, mut s) in world.query_mut::<&mut Shield>() {
                if e == entity {
                    *s = shield;
                    break;
                }
            }
        }
    }
}
