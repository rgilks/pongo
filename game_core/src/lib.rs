pub mod components;
pub mod map;
pub mod params;
pub mod resources;
pub mod systems;

pub use components::*;
pub use map::*;
pub use params::*;
pub use resources::*;

use hecs::World;
use systems::*;

/// Run the deterministic game simulation schedule
#[allow(clippy::too_many_arguments)]
pub fn step(
    world: &mut World,
    time: &mut Time,
    map: &GameMap,
    rng: &mut GameRng,
    score: &mut Score,
    events: &mut Events,
    config: &Config,
    net_queue: &mut NetQueue,
) {
    // Clamp dt to prevent large jumps
    let clamped_dt = time.dt.min(params::Params::MAX_DT);

    // Fixed micro-steps for stable physics
    let mut remaining_dt = clamped_dt;
    while remaining_dt > 0.0 {
        let step_dt = remaining_dt.min(params::Params::FIXED_DT);
        remaining_dt -= step_dt;

        let step_time = Time {
            dt: step_dt,
            now: time.now + (clamped_dt - remaining_dt),
        };

        // Clear events at start of frame
        events.clear();

        // Ingest inputs (creates MovementIntent/CombatIntent)
        ingest_inputs(world, net_queue);
        apply_shield_intents(world);

        // BotThink (handled externally for now)

        // Apply movement intents
        apply_movement_intent(world, &step_time);

        // Integrate motion
        integrate_motion(world, &step_time, map);

        // Shield update
        shield_update(world, &step_time);

        // Fire bolts
        fire_bolts(world, &step_time, events);

        // Spawn from events
        spawn_from_events(world, events);

        // Bolts step
        bolts_step(world, &step_time, map);

        // Resolve hits
        resolve_hits(world, events);

        // Apply damage
        apply_damage(world, events, score);

        // Handle eliminations (create respawn timers)
        let eliminated_ids: Vec<u16> = events.eliminated.clone();
        for player_id in eliminated_ids {
            // Find player entity
            let mut player_entity = None;
            for (entity, player) in world.query::<&Player>().iter() {
                if player.id == player_id {
                    player_entity = Some(entity);
                    break;
                }
            }
            if let Some(entity) = player_entity {
                // Add respawn timer
                let timer = RespawnTimer::new(Params::RESPAWN_DELAY);
                world.insert(entity, (timer,)).unwrap();
            }
        }

        // Respawn tick
        respawn_tick(world, &step_time, map, events);
        apply_respawns(world, events);

        // Pickups spawn
        pickups_spawn(world, &step_time, map, rng);

        // Pickups collect
        pickups_collect(world, events);

        // Apply pickup effects
        apply_pickup_effects(world, events);

        // Hill score tick
        hill_score_tick(world, &step_time, map, score, config);

        // Energy regen
        energy_regen(world, &step_time);

        // GC
        gc(world, &step_time);
    }

    // Update time
    time.now += clamped_dt;
}

/// Helper to create a player entity with all required components
pub fn create_player(
    world: &mut World,
    player_id: u16,
    avatar: u8,
    name_id: u8,
    pos: glam::Vec2,
) -> hecs::Entity {
    world.spawn((
        Player {
            id: player_id,
            avatar,
            name_id,
        },
        Transform2D::new(pos, 0.0),
        Velocity2D::new(glam::Vec2::ZERO),
        Health::new(),
        Energy::new(),
        Shield::new(),
        BoltMaxLevel::new(),
        BoltCooldown::new(),
        MovementIntent::new(),
        CombatIntent::new(),
    ))
}
