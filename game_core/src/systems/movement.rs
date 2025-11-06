use hecs::World;

use crate::components::*;
use crate::resources::*;
use crate::params::Params;

/// Apply movement intents to velocities
pub fn apply_movement_intent(world: &mut World, time: &Time) {
    // Collect entities with movement intents (deterministic: sort by entity ID)
    let mut entities: Vec<_> = world.query::<&MovementIntent>().iter().map(|(e, _)| e).collect();
    entities.sort_by_key(|e| e.id());

    for entity in entities {
        let intent = *world.get::<&MovementIntent>(entity).unwrap();
        let transform = *world.get::<&Transform2D>(entity).unwrap();
        
        // Apply turn
        let turn_amount = intent.turn * Params::TURN_RATE * time.dt;
        for (e, mut t) in world.query_mut::<&mut Transform2D>() {
            if e == entity {
                t.yaw += turn_amount;
                break;
            }
        }

        // Apply thrust to velocity
        let forward = transform.forward();
        let speed = intent.thrust * Params::MOVE_SPEED;
        let mut found = false;
        for (e, mut vel) in world.query_mut::<&mut Velocity2D>() {
            if e == entity {
                vel.vel = forward * speed;
                found = true;
                break;
            }
        }
        if !found {
            world.insert(entity, (Velocity2D::new(forward * speed),)).unwrap();
        }
    }
}

/// Integrate motion and handle collisions
pub fn integrate_motion(world: &mut World, time: &Time, map: &GameMap) {
    // Collect entities with transform and velocity (deterministic: sort by entity ID)
    let mut entities: Vec<_> = world.query::<(&Transform2D, &Velocity2D)>().iter().map(|(e, _)| e).collect();
    entities.sort_by_key(|e| e.id());

    for entity in entities {
        let transform = *world.get::<&Transform2D>(entity).unwrap();
        let vel = *world.get::<&Velocity2D>(entity).unwrap();
        
        // Integrate position
        let new_pos = transform.pos + vel.vel * time.dt;

        // Check collision with map blocks (circle vs AABB)
        let mut final_pos = new_pos;
        let radius = if world.get::<&Player>(entity).is_ok() {
            Params::PLAYER_RADIUS
        } else {
            0.0 // bolts have their own radius
        };

        for block in &map.map.blocks {
            if block.intersects_circle(final_pos, radius) {
                // Simple collision response: push out along shortest axis
                let center = (block.min + block.max) * 0.5;
                let size = block.max - block.min;
                let half_size = size * 0.5;
                
                let diff = final_pos - center;
                let overlap_x = (diff.x.abs() - half_size.x).max(0.0);
                let overlap_y = (diff.y.abs() - half_size.y).max(0.0);
                
                if overlap_x < overlap_y {
                    // Push along X
                    if diff.x > 0.0 {
                        final_pos.x = block.max.x + radius;
                    } else {
                        final_pos.x = block.min.x - radius;
                    }
                } else {
                    // Push along Y
                    if diff.y > 0.0 {
                        final_pos.y = block.max.y + radius;
                    } else {
                        final_pos.y = block.min.y - radius;
                    }
                }
            }
        }

        // Clamp to world bounds
        final_pos.x = final_pos.x.clamp(-Params::WORLD_BOUNDS, Params::WORLD_BOUNDS);
        final_pos.y = final_pos.y.clamp(-Params::WORLD_BOUNDS, Params::WORLD_BOUNDS);

        // Update position
        for (e, mut t) in world.query_mut::<&mut Transform2D>() {
            if e == entity {
                t.pos = final_pos;
                break;
            }
        }
    }
}
