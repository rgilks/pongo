/// Game tuning parameters (from spec §3)
#[derive(Debug, Clone, Copy)]
pub struct Params;

impl Params {
    // Movement
    pub const TURN_RATE: f32 = 210.0_f32.to_radians(); // 210°/s in rad/s
    pub const MOVE_SPEED: f32 = 6.5; // u/s
    pub const PLAYER_RADIUS: f32 = 0.6; // u

    // Energy
    pub const ENERGY_MAX: f32 = 100.0;
    pub const ENERGY_REGEN: f32 = 15.0; // per second

    // Bolt levels (costE / speed / damage / radius)
    pub const BOLT_COOLDOWN: f32 = 0.15; // 150ms
    pub const BOLT_LIFETIME: f32 = 1.6; // seconds

    pub const BOLT_L1_COST: f32 = 10.0;
    pub const BOLT_L1_SPEED: f32 = 10.0;
    pub const BOLT_L1_DMG: u8 = 1;
    pub const BOLT_L1_RADIUS: f32 = 0.25;

    pub const BOLT_L2_COST: f32 = 20.0;
    pub const BOLT_L2_SPEED: f32 = 13.0;
    pub const BOLT_L2_DMG: u8 = 2;
    pub const BOLT_L2_RADIUS: f32 = 0.30;

    pub const BOLT_L3_COST: f32 = 35.0;
    pub const BOLT_L3_SPEED: f32 = 16.0;
    pub const BOLT_L3_DMG: u8 = 3;
    pub const BOLT_L3_RADIUS: f32 = 0.35;

    // Shield
    pub const SHIELD_ARC: f32 = 120.0_f32.to_radians(); // ~120° frontal arc
    pub const SHIELD_MAX_DURATION: f32 = 0.6; // max up 0.6s
    pub const SHIELD_COOLDOWN: f32 = 0.4; // 0.4s cooldown

    pub const SHIELD_S1_DRAIN: f32 = 8.0; // E per second
    pub const SHIELD_S2_DRAIN: f32 = 16.0;
    pub const SHIELD_S3_DRAIN: f32 = 28.0;

    // Pickups
    pub const PICKUP_RESPAWN_MIN: f32 = 8.0; // seconds
    pub const PICKUP_RESPAWN_MAX: f32 = 16.0; // seconds
    pub const PICKUP_STALE_TIME: f32 = 20.0; // despawn after 20s

    // Hill
    pub const HILL_RADIUS: f32 = 3.0; // u
    pub const HILL_POINTS_PER_SEC: u16 = 1;
    pub const HILL_POINTS_TO_WIN: u16 = 100;
    pub const HILL_ROTATION_INTERVAL: f32 = 60.0; // seconds

    // Respawn
    pub const RESPAWN_DELAY: f32 = 2.0; // seconds
    pub const RESPAWN_SHIELD_LEVEL: u8 = 2;
    pub const RESPAWN_SHIELD_DURATION: f32 = 0.5; // seconds

    // Match
    pub const MATCH_TIME_S: f32 = 300.0; // 5 minutes
    pub const MAX_PLAYERS: u8 = 8;
    pub const TARGET_ACTORS: u8 = 6;

    // Physics
    pub const FIXED_DT: f32 = 0.01; // 10ms fixed timestep
    pub const MAX_DT: f32 = 0.1; // clamp dt to prevent large jumps

    // World bounds
    pub const WORLD_BOUNDS: f32 = 32.0; // ±32u
}

