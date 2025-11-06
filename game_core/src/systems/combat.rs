use hecs::World;

use crate::components::*;
use crate::resources::*;
use crate::params::Params;

/// Update shield state, drain energy, handle timers
pub fn shield_update(world: &mut World, time: &Time) {
    // Collect entities with shield and energy (deterministic: sort by entity ID)
    let mut entities: Vec<_> = world.query::<(&Shield, &Energy)>().iter().map(|(e, _)| e).collect();
    entities.sort_by_key(|e| e.id());

    for entity in entities {
        let shield = *world.get::<&Shield>(entity).unwrap();
        let energy = *world.get::<&Energy>(entity).unwrap();
        let mut shield = shield;
        let mut energy = energy;

        // Update cooldown
        if shield.cooldown > 0.0 {
            shield.cooldown -= time.dt;
        }

        // Update active shield
        if shield.active > 0 {
            shield.t_left -= time.dt;
            
            // Drain energy
            let drain_rate = match shield.active {
                1 => Params::SHIELD_S1_DRAIN,
                2 => Params::SHIELD_S2_DRAIN,
                3 => Params::SHIELD_S3_DRAIN,
                _ => 0.0,
            };
            energy.spend(drain_rate * time.dt);

            // Deactivate if time expired or energy depleted
            if shield.t_left <= 0.0 || energy.cur <= 0.0 {
                shield.active = 0;
                shield.cooldown = Params::SHIELD_COOLDOWN;
            }
        }

        // Update components
        for (e, mut s) in world.query_mut::<&mut Shield>() {
            if e == entity {
                *s = shield;
                break;
            }
        }
        for (e, mut e_comp) in world.query_mut::<&mut Energy>() {
            if e == entity {
                *e_comp = energy;
                break;
            }
        }
    }
}

/// Process fire bolt intents
pub fn fire_bolts(world: &mut World, _time: &Time, events: &mut Events) {
    // Collect entities with combat intents (deterministic: sort by entity ID)
    let mut entities: Vec<_> = world.query::<(&CombatIntent, &BoltCooldown, &Energy, &Transform2D, &Player)>()
        .iter()
        .map(|(e, _)| e)
        .collect();
    entities.sort_by_key(|e| e.id());

    for entity in entities {
        let intent = *world.get::<&CombatIntent>(entity).unwrap();
        let cooldown = *world.get::<&BoltCooldown>(entity).unwrap();
        let energy = *world.get::<&Energy>(entity).unwrap();
        let transform = *world.get::<&Transform2D>(entity).unwrap();
        let player = *world.get::<&Player>(entity).unwrap();

        if intent.bolt_level == 0 || !cooldown.can_fire() {
            continue;
        }

        let level = intent.bolt_level;
        
        // Check if player has bolt upgrade level
        let bolt_max = world.get::<&crate::components::BoltMaxLevel>(entity)
            .map(|b| b.level)
            .unwrap_or(1);
        
        if level > bolt_max {
            continue;
        }
        
        let cost = match level {
            1 => Params::BOLT_L1_COST,
            2 => Params::BOLT_L2_COST,
            3 => Params::BOLT_L3_COST,
            _ => continue,
        };

        if !energy.can_afford(cost) {
            continue;
        }
        
        // Fire bolt
        let speed = match level {
            1 => Params::BOLT_L1_SPEED,
            2 => Params::BOLT_L2_SPEED,
            3 => Params::BOLT_L3_SPEED,
            _ => continue,
        };

        let forward = transform.forward();
        let vel = forward * speed;
        let pos = transform.pos + forward * (Params::PLAYER_RADIUS + 0.1); // spawn slightly ahead

        events.spawn_bolt.push(SpawnBoltEvent {
            owner: player.id,
            level,
            pos,
            vel,
        });

        // Spend energy and reset cooldown
        let mut new_energy = energy;
        new_energy.spend(cost);
        for (e, mut e_comp) in world.query_mut::<&mut Energy>() {
            if e == entity {
                *e_comp = new_energy;
                break;
            }
        }
        
        let mut new_cooldown = BoltCooldown::new();
        new_cooldown.t_left = Params::BOLT_COOLDOWN;
        for (e, mut c) in world.query_mut::<&mut BoltCooldown>() {
            if e == entity {
                *c = new_cooldown;
                break;
            }
        }
    }
}

/// Spawn bolts from events
pub fn spawn_from_events(world: &mut World, events: &Events) {
    for event in &events.spawn_bolt {
        let bolt = Bolt::new(event.level, event.owner);
        let lifetime = Lifetime::new(Params::BOLT_LIFETIME);
        let transform = Transform2D::new(event.pos, event.vel.normalize().y.atan2(event.vel.x));
        let velocity = Velocity2D::new(event.vel);

        world.spawn((bolt, lifetime, transform, velocity));
    }
}

/// Step bolts forward, check collisions with blocks, expire
pub fn bolts_step(world: &mut World, time: &Time, map: &GameMap) {
    // Collect bolt entities (deterministic: sort by entity ID)
    let mut entities: Vec<_> = world.query::<(&Bolt, &Lifetime, &Transform2D)>()
        .iter()
        .map(|(e, _)| e)
        .collect();
    entities.sort_by_key(|e| e.id());

    let mut to_remove = Vec::new();

    for entity in entities {
        let lifetime = *world.get::<&Lifetime>(entity).unwrap();
        let mut lifetime = lifetime;
        let transform = *world.get::<&Transform2D>(entity).unwrap();
        let bolt = *world.get::<&Bolt>(entity).unwrap();

        // Update lifetime
        lifetime.t_left -= time.dt;

        if lifetime.is_expired() {
            to_remove.push(entity);
            continue;
        }

        // Check collision with blocks
        let mut should_remove = false;
        for block in &map.map.blocks {
            if block.intersects_circle(transform.pos, bolt.radius) {
                should_remove = true;
                break;
            }
        }

        if should_remove {
            to_remove.push(entity);
            continue;
        }

        // Update lifetime
        for (e, mut l) in world.query_mut::<&mut Lifetime>() {
            if e == entity {
                *l = lifetime;
                break;
            }
        }
    }

    // Remove expired/collided bolts
    for entity in to_remove {
        let _ = world.despawn(entity);
    }
}

/// Resolve hits: bolts vs players
pub fn resolve_hits(world: &mut World, events: &mut Events) {
    // Collect bolts (deterministic: sort by entity ID)
    let mut bolt_entities: Vec<_> = world.query::<(&Bolt, &Transform2D)>()
        .iter()
        .map(|(e, _)| e)
        .collect();
    bolt_entities.sort_by_key(|e| e.id());

    // Collect players (deterministic: sort by entity ID)
    let mut player_entities: Vec<_> = world.query::<(&Player, &Transform2D, &Shield, &Health)>()
        .iter()
        .map(|(e, _)| e)
        .collect();
    player_entities.sort_by_key(|e| e.id());

    let mut bolts_to_remove = Vec::new();

    for bolt_entity in &bolt_entities {
        let bolt = *world.get::<&Bolt>(*bolt_entity).unwrap();
        let bolt_transform = *world.get::<&Transform2D>(*bolt_entity).unwrap();
        let owner_id = bolt.owner;

        for player_entity in &player_entities {
            let player = *world.get::<&Player>(*player_entity).unwrap();
            let player_transform = *world.get::<&Transform2D>(*player_entity).unwrap();
            let shield = *world.get::<&Shield>(*player_entity).unwrap();
            let health = *world.get::<&Health>(*player_entity).unwrap();

            // Don't hit owner
            if player.id == owner_id {
                continue;
            }

            // Don't hit eliminated players
            if health.is_eliminated() {
                continue;
            }

            // Check collision (circle vs circle)
            let dist = (bolt_transform.pos - player_transform.pos).length();
            let collision_dist = bolt.radius + Params::PLAYER_RADIUS;

            if dist < collision_dist {
                // Check shield
                let mut damage = bolt.dmg;
                
                if shield.is_active() {
                    // Check if bolt is in shield arc
                    let to_bolt = (bolt_transform.pos - player_transform.pos).normalize();
                    let forward = player_transform.forward();
                    let angle = to_bolt.dot(forward).acos();
                    
                    if angle <= Params::SHIELD_ARC * 0.5 {
                        // Shield blocks: damage = max(0, BoltLevel - ShieldLevel)
                        damage = damage.saturating_sub(shield.active as u8);
                    }
                }

                if damage > 0 {
                    events.apply_damage.push(DamageEvent {
                        target: player.id,
                        amount: damage,
                        source: owner_id,
                    });
                }

                bolts_to_remove.push(*bolt_entity);
                break;
            }
        }
    }

    // Remove hit bolts
    for entity in bolts_to_remove {
        let _ = world.despawn(entity);
    }
}

/// Apply damage and handle eliminations
pub fn apply_damage(world: &mut World, events: &mut Events, score: &mut Score) {
    for damage_event in &events.apply_damage {
        // Find player entity
        let mut player_entity = None;
        for (entity, player) in world.query::<&Player>().iter() {
            if player.id == damage_event.target {
                player_entity = Some(entity);
                break;
            }
        }

        if let Some(entity) = player_entity {
            let health = *world.get::<&Health>(entity).unwrap();
            let mut health = health;
            health.damage = (health.damage + damage_event.amount).min(3);
            
            if health.is_eliminated() {
                events.eliminated.push(damage_event.target);
                
                // Increment elimination count for source
                *score.eliminations.entry(damage_event.source).or_insert(0) += 1;
            }
            
            // Update health
            for (e, mut h) in world.query_mut::<&mut Health>() {
                if e == entity {
                    *h = health;
                    break;
                }
            }
        }
    }
}
