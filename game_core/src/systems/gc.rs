use hecs::World;

use crate::components::*;
use crate::resources::*;

/// Garbage collection: despawn expired entities
pub fn gc(world: &mut World, time: &Time) {
    let mut to_remove = Vec::new();

    // Find expired lifetimes
    for (entity, lifetime) in world.query::<&Lifetime>().iter() {
        let mut lifetime = *lifetime;
        lifetime.t_left -= time.dt;

        if lifetime.is_expired() {
            to_remove.push(entity);
        }
    }

    // Remove expired entities
    for entity in to_remove {
        let _ = world.despawn(entity);
    }
}
