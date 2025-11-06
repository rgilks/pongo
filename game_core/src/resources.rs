use glam::Vec2;
use hecs::Entity;
use rand::Rng as RandRng;
use std::collections::HashMap;

use crate::components::PickupKind;
use crate::map::Map;
use crate::params::Params;

/// Game parameters
#[derive(Debug)]
pub struct GameParams {
    pub params: Params,
}

impl Default for GameParams {
    fn default() -> Self {
        Self { params: Params }
    }
}

impl GameParams {
    pub fn new() -> Self {
        Self::default()
    }
}

/// Map resource
#[derive(Debug, Clone)]
pub struct GameMap {
    pub map: Map,
}

impl GameMap {
    pub fn new(map: Map) -> Self {
        Self { map }
    }
}

/// Time resource
#[derive(Debug, Clone, Copy, Default)]
pub struct Time {
    pub dt: f32,
    pub now: f32, // total elapsed time
}

impl Time {
    pub fn new() -> Self {
        Self::default()
    }
}

/// Deterministic RNG (server authority)
pub struct GameRng {
    rng: rand::rngs::ThreadRng,
}

impl Default for GameRng {
    fn default() -> Self {
        Self {
            rng: rand::thread_rng(),
        }
    }
}

impl GameRng {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn gen(&mut self) -> f32 {
        self.rng.gen()
    }

    pub fn gen_range(&mut self, min: f32, max: f32) -> f32 {
        self.rng.gen_range(min..max)
    }
}

/// Score tracking
#[derive(Debug, Default)]
pub struct Score {
    pub hill_points: HashMap<u16, u16>,  // player_id -> points
    pub eliminations: HashMap<u16, u16>, // player_id -> count
    pub hill_points_fractional: HashMap<u16, f32>, // player_id -> fractional points accumulator
}

impl Score {
    pub fn new() -> Self {
        Self::default()
    }
}

/// Network queue (for server/client)
#[derive(Debug, Default)]
pub struct NetQueue {
    pub inputs: Vec<InputEvent>,
    pub acks: Vec<u32>, // snapshot IDs
}

impl NetQueue {
    pub fn new() -> Self {
        Self::default()
    }
}

/// Input event
#[derive(Debug, Clone)]
pub struct InputEvent {
    pub player_id: u16,
    pub seq: u32,
    pub t_ms: u32,
    pub thrust: f32,
    pub turn: f32,
    pub bolt_level: u8,
    pub shield_level: u8,
}

/// Match configuration
#[derive(Debug, Clone)]
pub struct Config {
    pub objective_on: bool,
    pub target_actors: u8,
    pub max_players: u8,
    pub hill_points_to_win: u16,
    pub match_time_s: f32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            objective_on: true,
            target_actors: Params::TARGET_ACTORS,
            max_players: Params::MAX_PLAYERS,
            hill_points_to_win: Params::HILL_POINTS_TO_WIN,
            match_time_s: Params::MATCH_TIME_S,
        }
    }
}

impl Config {
    pub fn new() -> Self {
        Self::default()
    }
}

/// Game events (collected during frame, processed after)
#[derive(Debug, Default)]
pub struct Events {
    pub spawn_bolt: Vec<SpawnBoltEvent>,
    pub apply_damage: Vec<DamageEvent>,
    pub eliminated: Vec<u16>, // player IDs
    pub pickup_taken: Vec<PickupTakenEvent>,
    pub respawn: Vec<RespawnEvent>,
}

impl Events {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        self.spawn_bolt.clear();
        self.apply_damage.clear();
        self.eliminated.clear();
        self.pickup_taken.clear();
        self.respawn.clear();
    }
}

/// Spawn bolt event
#[derive(Debug, Clone)]
pub struct SpawnBoltEvent {
    pub owner: u16,
    pub level: u8,
    pub pos: Vec2,
    pub vel: Vec2,
}

/// Damage event
#[derive(Debug, Clone)]
pub struct DamageEvent {
    pub target: u16,
    pub amount: u8,
    pub source: u16, // bolt owner
}

/// Pickup taken event
#[derive(Debug, Clone)]
pub struct PickupTakenEvent {
    pub player_id: u16,
    pub pickup_entity: Entity,
    pub kind: PickupKind,
}

/// Respawn event
#[derive(Debug, Clone)]
pub struct RespawnEvent {
    pub player_id: u16,
    pub spawn_pos: Vec2,
}
