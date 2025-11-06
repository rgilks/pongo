use glam::Vec2;

/// 2D transform: position and yaw (rotation around Z axis)
#[derive(Debug, Clone, Copy)]
pub struct Transform2D {
    pub pos: Vec2,
    pub yaw: f32, // radians, 0 = +X, π/2 = +Y
}

impl Transform2D {
    pub fn new(pos: Vec2, yaw: f32) -> Self {
        Self { pos, yaw }
    }

    pub fn forward(&self) -> Vec2 {
        Vec2::new(self.yaw.cos(), self.yaw.sin())
    }
}

/// 2D velocity
#[derive(Debug, Clone, Copy)]
pub struct Velocity2D {
    pub vel: Vec2,
}

impl Velocity2D {
    pub fn new(vel: Vec2) -> Self {
        Self { vel }
    }
}

/// Player identity
#[derive(Debug, Clone, Copy)]
pub struct Player {
    pub id: u16,
    pub avatar: u8,
    pub name_id: u8,
}

/// Health as damage slots (0..3)
#[derive(Debug, Clone, Copy)]
pub struct Health {
    pub damage: u8, // 0 = full health, 3 = eliminated
}

impl Health {
    pub fn new() -> Self {
        Self { damage: 0 }
    }

    pub fn is_eliminated(&self) -> bool {
        self.damage >= 3
    }
}

/// Energy resource (0.0..100.0)
#[derive(Debug, Clone, Copy)]
pub struct Energy {
    pub cur: f32,
}

impl Energy {
    pub fn new() -> Self {
        Self { cur: 100.0 }
    }

    pub fn can_afford(&self, cost: f32) -> bool {
        self.cur >= cost
    }

    pub fn spend(&mut self, amount: f32) {
        self.cur = (self.cur - amount).max(0.0);
    }
}

/// Shield component
#[derive(Debug, Clone, Copy)]
pub struct Shield {
    pub max: u8,        // max level (0..3, 0 = not unlocked)
    pub active: u8,     // current active level (0..3)
    pub t_left: f32,    // time remaining (max 0.6s)
    pub cooldown: f32,  // cooldown timer (0.4s after deactivate)
}

impl Shield {
    pub fn new() -> Self {
        Self {
            max: 0,
            active: 0,
            t_left: 0.0,
            cooldown: 0.0,
        }
    }

    pub fn can_activate(&self, level: u8) -> bool {
        level > 0 && level <= self.max && self.cooldown <= 0.0
    }

    pub fn is_active(&self) -> bool {
        self.active > 0
    }
}

/// Bolt projectile
#[derive(Debug, Clone, Copy)]
pub struct Bolt {
    pub level: u8,      // 1..3
    pub dmg: u8,        // damage amount
    pub radius: f32,    // collision radius
    pub owner: u16,     // player ID who fired it
}

impl Bolt {
    pub fn new(level: u8, owner: u16) -> Self {
        let (dmg, radius) = match level {
            1 => (1, 0.25),
            2 => (2, 0.30),
            3 => (3, 0.35),
            _ => (1, 0.25),
        };
        Self {
            level,
            dmg,
            radius,
            owner,
        }
    }
}

/// Lifetime component for temporary entities
#[derive(Debug, Clone, Copy)]
pub struct Lifetime {
    pub t_left: f32,
}

impl Lifetime {
    pub fn new(duration: f32) -> Self {
        Self { t_left: duration }
    }

    pub fn is_expired(&self) -> bool {
        self.t_left <= 0.0
    }
}

/// Pickup item kind
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickupKind {
    Health,
    BoltUpgrade,
    ShieldModule,
}

/// Pickup item
#[derive(Debug, Clone, Copy)]
pub struct Pickup {
    pub kind: PickupKind,
}

/// Spawn pad for pickups
#[derive(Debug, Clone)]
pub struct SpawnPad {
    pub kind: PickupKind,
    pub respawn_min: f32,
    pub respawn_max: f32,
    pub t_until: f32,
}

impl SpawnPad {
    pub fn new(kind: PickupKind, respawn_min: f32, respawn_max: f32) -> Self {
        Self {
            kind,
            respawn_min,
            respawn_max,
            t_until: 0.0,
        }
    }
}

/// Hill zone (King of the Hill objective)
#[derive(Debug, Clone, Copy)]
pub struct HillZone {
    pub center: Vec2,
    pub r: f32, // radius
}

impl HillZone {
    pub fn new(center: Vec2, r: f32) -> Self {
        Self { center, r }
    }

    pub fn contains(&self, pos: Vec2) -> bool {
        (pos - self.center).length() <= self.r
    }
}

/// Bot AI brain
#[derive(Debug, Clone)]
pub struct BotBrain {
    pub state: BotState,
    pub reaction_ms: f32,
    pub aim_err_deg: f32,
    pub fire_min_gap: f32,
    pub retreat_threshold: f32,
    pub hill_obsession: f32,
    pub last_fire_t: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BotState {
    SeekPickups,
    ContestHill,
    EvadeBolt,
    Engage,
}

impl BotBrain {
    pub fn new() -> Self {
        Self {
            state: BotState::SeekPickups,
            reaction_ms: 100.0,
            aim_err_deg: 5.0,
            fire_min_gap: 0.2,
            retreat_threshold: 0.5,
            hill_obsession: 0.7,
            last_fire_t: -1.0,
        }
    }
}

/// Movement intent (from input)
#[derive(Debug, Clone, Copy)]
pub struct MovementIntent {
    pub thrust: f32, // -1.0 (back) to +1.0 (forward)
    pub turn: f32,   // -1.0 (left) to +1.0 (right)
}

impl MovementIntent {
    pub fn new() -> Self {
        Self {
            thrust: 0.0,
            turn: 0.0,
        }
    }
}

/// Combat intent (from input)
#[derive(Debug, Clone, Copy)]
pub struct CombatIntent {
    pub bolt_level: u8,  // 0 = none, 1..3 = fire
    pub shield_level: u8, // 0 = none, 1..3 = activate
}

impl CombatIntent {
    pub fn new() -> Self {
        Self {
            bolt_level: 0,
            shield_level: 0,
        }
    }
}

/// Bolt cooldown tracker
#[derive(Debug, Clone, Copy)]
pub struct BoltCooldown {
    pub t_left: f32,
}

impl BoltCooldown {
    pub fn new() -> Self {
        Self { t_left: 0.0 }
    }

    pub fn can_fire(&self) -> bool {
        self.t_left <= 0.0
    }
}

/// Respawn timer
#[derive(Debug, Clone, Copy)]
pub struct RespawnTimer {
    pub t_left: f32,
}

impl RespawnTimer {
    pub fn new(duration: f32) -> Self {
        Self { t_left: duration }
    }

    pub fn is_ready(&self) -> bool {
        self.t_left <= 0.0
    }
}

/// Bolt upgrade level (max level player can fire)
#[derive(Debug, Clone, Copy)]
pub struct BoltMaxLevel {
    pub level: u8, // 1..3
}

impl BoltMaxLevel {
    pub fn new() -> Self {
        Self { level: 1 } // Start with L1 only
    }
}

