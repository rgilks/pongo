use hecs::World;

use crate::components::*;
use crate::params::Params;
use crate::resources::*;

/// Regenerate energy
pub fn energy_regen(world: &mut World, time: &Time) {
    // Collect all energy components (deterministic: sort by entity ID)
    let mut entities: Vec<_> = world.query::<&Energy>().iter().map(|(e, _)| e).collect();
    entities.sort_by_key(|e| e.id());

    for entity in entities {
        let energy = *world.get::<&Energy>(entity).unwrap();
        let mut energy = energy;
        energy.cur = (energy.cur + Params::ENERGY_REGEN * time.dt).min(Params::ENERGY_MAX);

        for (e, e_comp) in world.query_mut::<&mut Energy>() {
            if e == entity {
                *e_comp = energy;
                break;
            }
        }
    }
}
