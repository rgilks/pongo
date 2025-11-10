use glam::Vec2;

/// Paddle component - represents a player's paddle
#[derive(Debug, Clone, Copy)]
pub struct Paddle {
    pub player_id: u8, // 0 = left, 1 = right
    pub y: f32,        // Y position (clamped to arena)
}

impl Paddle {
    pub fn new(player_id: u8, y: f32) -> Self {
        Self { player_id, y }
    }
}

/// Ball component - the pong ball
#[derive(Debug, Clone, Copy)]
pub struct Ball {
    pub pos: Vec2,
    pub vel: Vec2,
}

impl Ball {
    pub fn new(pos: Vec2, vel: Vec2) -> Self {
        Self { pos, vel }
    }

    /// Reset ball to center with random direction
    pub fn reset(&mut self, speed: f32, rng: &mut crate::GameRng) {
        self.pos = Vec2::new(16.0, 12.0); // Center of 32x24 arena

        // Random angle between -45° and 45°, or 135° and 225°
        use rand::Rng;
        let right = rng.0.gen_bool(0.5);
        let angle: f32 = if right {
            rng.0.gen_range(-0.785..0.785) // -45° to 45° in radians
        } else {
            rng.0.gen_range(2.356..3.927) // 135° to 225° in radians
        };

        self.vel = Vec2::new(angle.cos(), angle.sin()) * speed;
    }
}

/// Movement intent for paddle
#[derive(Debug, Clone, Copy, Default)]
pub struct PaddleIntent {
    pub dir: i8, // -1 = up, 0 = stop, 1 = down
}

impl PaddleIntent {
    pub fn new() -> Self {
        Self::default()
    }
}
