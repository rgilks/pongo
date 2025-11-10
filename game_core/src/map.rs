use glam::Vec2;

/// Simple Pong arena - just the dimensions
#[derive(Debug, Clone)]
pub struct GameMap {
    pub width: f32,
    pub height: f32,
}

impl GameMap {
    /// Create standard Pong arena (32 x 24)
    pub fn new() -> Self {
        Self {
            width: crate::params::Params::ARENA_WIDTH,
            height: crate::params::Params::ARENA_HEIGHT,
        }
    }

    /// Get spawn position for paddle based on player ID
    pub fn paddle_spawn(&self, player_id: u8) -> Vec2 {
        let x = if player_id == 0 {
            1.0 // Left paddle
        } else {
            self.width - 1.0 // Right paddle
        };
        let y = self.height / 2.0; // Center vertically
        Vec2::new(x, y)
    }

    /// Get ball spawn position (center of arena)
    pub fn ball_spawn(&self) -> Vec2 {
        Vec2::new(self.width / 2.0, self.height / 2.0)
    }

    /// Check if Y position is within arena bounds
    pub fn is_valid_y(&self, y: f32, half_height: f32) -> bool {
        y >= half_height && y <= self.height - half_height
    }

    /// Clamp Y position to arena bounds
    pub fn clamp_y(&self, y: f32, half_height: f32) -> f32 {
        y.clamp(half_height, self.height - half_height)
    }
}

impl Default for GameMap {
    fn default() -> Self {
        Self::new()
    }
}
