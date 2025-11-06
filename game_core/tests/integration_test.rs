use game_core::*;
use hecs::World;
use glam::Vec2;

// Fix Rng reference
use game_core::resources::GameRng as Rng;

#[test]
fn test_movement() {
    let mut world = World::new();
    let mut time = Time::new();
    let map = GameMap::new(Map::test_map());
    let mut rng = Rng::new();
    let mut score = Score::new();
    let mut events = Events::new();
    let config = Config::new();
    let mut net_queue = NetQueue::new();

    // Create a player
    let player_entity = create_player(&mut world, 1, 0, 0, Vec2::new(0.0, 0.0));

    // Set movement intent
    let intent = MovementIntent {
        thrust: 1.0, // forward
        turn: 0.0,
    };
    world.insert(player_entity, (intent,)).unwrap();

    // Step simulation
    time.dt = 0.1;
    step(
        &mut world,
        &mut time,
        &map,
        &mut rng,
        &mut score,
        &mut events,
        &config,
        &mut net_queue,
    );

    // Check position changed
    let transform = world.get::<&Transform2D>(player_entity).unwrap();
    assert!(transform.pos.length() > 0.0);
}

#[test]
fn test_bolt_fire() {
    let mut world = World::new();
    let mut time = Time::new();
    let map = GameMap::new(Map::test_map());
    let mut rng = Rng::new();
    let mut score = Score::new();
    let mut events = Events::new();
    let config = Config::new();
    let mut net_queue = NetQueue::new();

    // Create a player
    let player_entity = create_player(&mut world, 1, 0, 0, Vec2::new(0.0, 0.0));

    // Set combat intent to fire bolt
    let combat = CombatIntent {
        bolt_level: 1,
        shield_level: 0,
    };
    world.insert(player_entity, (combat,)).unwrap();

    // Step simulation
    time.dt = 0.1;
    step(
        &mut world,
        &mut time,
        &map,
        &mut rng,
        &mut score,
        &mut events,
        &config,
        &mut net_queue,
    );

    // Check bolt was spawned
    let mut bolt_count = 0;
    for (_, _) in world.query::<&Bolt>().iter() {
        bolt_count += 1;
    }
    assert_eq!(bolt_count, 1);
}

#[test]
fn test_energy_drain() {
    let mut world = World::new();
    let mut time = Time::new();
    let map = GameMap::new(Map::test_map());
    let mut rng = Rng::new();
    let mut score = Score::new();
    let mut events = Events::new();
    let config = Config::new();
    let mut net_queue = NetQueue::new();

    // Create a player
    let player_entity = create_player(&mut world, 1, 0, 0, Vec2::new(0.0, 0.0));
    let initial_energy = world.get::<&Energy>(player_entity).unwrap().cur;

    // Fire bolt (costs energy)
    let combat = CombatIntent {
        bolt_level: 1,
        shield_level: 0,
    };
    world.insert(player_entity, (combat,)).unwrap();

    time.dt = 0.1;
    step(
        &mut world,
        &mut time,
        &map,
        &mut rng,
        &mut score,
        &mut events,
        &config,
        &mut net_queue,
    );

    // Check energy decreased
    let energy = world.get::<&Energy>(player_entity).unwrap();
    assert!(energy.cur < initial_energy);
}

#[test]
fn test_health_damage() {
    let mut world = World::new();
    let mut time = Time::new();
    let map = GameMap::new(Map::test_map());
    let mut rng = Rng::new();
    let mut score = Score::new();
    let mut events = Events::new();
    let config = Config::new();
    let mut net_queue = NetQueue::new();

    // Create two players
    let player1 = create_player(&mut world, 1, 0, 0, Vec2::new(0.0, 0.0));
    let player2 = create_player(&mut world, 2, 0, 0, Vec2::new(1.0, 0.0)); // Close to player1

    // Player1 fires bolt
    let combat = CombatIntent {
        bolt_level: 1,
        shield_level: 0,
    };
    world.insert(player1, (combat,)).unwrap();

    // Step until bolt hits
    for _ in 0..20 {
        time.dt = 0.1;
        step(
            &mut world,
            &mut time,
            &map,
            &mut rng,
            &mut score,
            &mut events,
            &config,
            &mut net_queue,
        );
    }

    // Check player2 took damage
    let health = world.get::<&Health>(player2).unwrap();
    assert!(health.damage > 0);
}

#[test]
fn test_elimination() {
    let mut world = World::new();
    let mut time = Time::new();
    let map = GameMap::new(Map::test_map());
    let mut rng = Rng::new();
    let mut score = Score::new();
    let mut events = Events::new();
    let config = Config::new();
    let mut net_queue = NetQueue::new();

    // Create two players
    let player1 = create_player(&mut world, 1, 0, 0, Vec2::new(0.0, 0.0));
    let player2 = create_player(&mut world, 2, 0, 0, Vec2::new(1.0, 0.0));

    // Set player2 to 2 damage (one hit from elimination)
    for (e, mut health) in world.query_mut::<&mut Health>() {
        if e == player2 {
            health.damage = 2;
            break;
        }
    }

    // Player1 fires L1 bolt (1 damage)
    let combat = CombatIntent {
        bolt_level: 1,
        shield_level: 0,
    };
    world.insert(player1, (combat,)).unwrap();

    // Step until bolt hits
    for _ in 0..20 {
        time.dt = 0.1;
        step(
            &mut world,
            &mut time,
            &map,
            &mut rng,
            &mut score,
            &mut events,
            &config,
            &mut net_queue,
        );
    }

    // Check player2 was eliminated
    let health = world.get::<&Health>(player2).unwrap();
    assert!(health.is_eliminated());
    
    // Check respawn timer was created
    assert!(world.get::<&RespawnTimer>(player2).is_ok());
}

#[test]
fn test_pickup_collection() {
    let mut world = World::new();
    let mut time = Time::new();
    let map = GameMap::new(Map::test_map());
    let mut rng = Rng::new();
    let mut score = Score::new();
    let mut events = Events::new();
    let config = Config::new();
    let mut net_queue = NetQueue::new();

    // Create a player
    let player_entity = create_player(&mut world, 1, 0, 0, Vec2::new(0.0, 0.0));

    // Create a pickup very close to the player (within collision radius)
    let pickup_pos = Vec2::new(0.3, 0.0); // Very close to player (radius 0.6 + pickup 0.3 = 0.9, so 0.3 is within range)
    let pickup = Pickup {
        kind: PickupKind::Health,
    };
    let transform = Transform2D::new(pickup_pos, 0.0);
    world.spawn((pickup, transform));

    // Step simulation multiple times to ensure collision
    time.dt = 0.1;
    for _ in 0..5 {
        step(
            &mut world,
            &mut time,
            &map,
            &mut rng,
            &mut score,
            &mut events,
            &config,
            &mut net_queue,
        );
    }

    // Check pickup was collected
    let mut pickup_count = 0;
    for (_, _) in world.query::<&Pickup>().iter() {
        pickup_count += 1;
    }
    assert_eq!(pickup_count, 0);
}

#[test]
fn test_hill_scoring() {
    let mut world = World::new();
    let mut time = Time::new();
    let map = GameMap::new(Map::test_map());
    let mut rng = Rng::new();
    let mut score = Score::new();
    let mut events = Events::new();
    let mut config = Config::new();
    config.objective_on = true;
    let mut net_queue = NetQueue::new();

    // Create a player at hill center
    let player_entity = create_player(&mut world, 1, 0, 0, map.map.hill_center);

    // Step for multiple frames to accumulate points
    for _ in 0..10 {
        time.dt = 0.1;
        step(
            &mut world,
            &mut time,
            &map,
            &mut rng,
            &mut score,
            &mut events,
            &config,
            &mut net_queue,
        );
    }

    // Check player earned points (should have at least 1 point after 1 second of being in hill)
    assert!(score.hill_points.get(&1).unwrap_or(&0) > &0);
}

