/// Time resource for tracking simulation time
#[derive(Debug, Clone, Copy)]
pub struct Time {
    pub dt: f32,  // Delta time for this step
    pub now: f32, // Total elapsed time
}

impl Time {
    pub fn new(dt: f32, now: f32) -> Self {
        Self { dt, now }
    }
}

impl Default for Time {
    fn default() -> Self {
        Self {
            dt: 0.016,
            now: 0.0,
        }
    }
}

/// Game score tracking
#[derive(Debug, Clone, Copy, Default)]
pub struct Score {
    pub left: u8,  // Left player score
    pub right: u8, // Right player score
}

impl Score {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn increment_left(&mut self) {
        self.left += 1;
    }

    pub fn increment_right(&mut self) {
        self.right += 1;
    }

    pub fn has_winner(&self, win_score: u8) -> Option<u8> {
        if self.left >= win_score {
            Some(0) // Left player wins
        } else if self.right >= win_score {
            Some(1) // Right player wins
        } else {
            None
        }
    }
}

/// Game configuration
#[derive(Debug, Clone)]
pub struct Config {
    pub arena_width: f32,
    pub arena_height: f32,
    pub paddle_width: f32,
    pub paddle_height: f32,
    pub paddle_speed: f32,
    pub ball_radius: f32,
    pub ball_speed_initial: f32,
    pub ball_speed_max: f32,
    pub ball_speed_increase: f32, // Multiplier on paddle hit
    pub win_score: u8,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            arena_width: 32.0,
            arena_height: 24.0,
            paddle_width: 1.0,
            paddle_height: 4.0,
            paddle_speed: 8.0,
            ball_radius: 0.5,
            ball_speed_initial: 8.0,
            ball_speed_max: 16.0,
            ball_speed_increase: 1.05,
            win_score: 11,
        }
    }
}

impl Config {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get X position for paddle based on player ID
    pub fn paddle_x(&self, player_id: u8) -> f32 {
        if player_id == 0 {
            1.0 // Left paddle
        } else {
            self.arena_width - 1.0 // Right paddle
        }
    }

    /// Clamp paddle Y to arena bounds
    pub fn clamp_paddle_y(&self, y: f32) -> f32 {
        let half_height = self.paddle_height / 2.0;
        y.clamp(half_height, self.arena_height - half_height)
    }
}

/// Random number generator
pub struct GameRng(pub rand::rngs::StdRng);

impl GameRng {
    pub fn new(seed: u64) -> Self {
        use rand::SeedableRng;
        Self(rand::rngs::StdRng::seed_from_u64(seed))
    }
}

impl Default for GameRng {
    fn default() -> Self {
        Self::new(12345)
    }
}

/// Events that occurred during this frame
#[derive(Debug, Clone, Default)]
pub struct Events {
    pub left_scored: bool,
    pub right_scored: bool,
    pub ball_hit_paddle: bool,
    pub ball_hit_wall: bool,
}

impl Events {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        self.left_scored = false;
        self.right_scored = false;
        self.ball_hit_paddle = false;
        self.ball_hit_wall = false;
    }
}

/// Network input queue (placeholder for network inputs)
#[derive(Debug, Clone, Default)]
pub struct NetQueue {
    pub inputs: Vec<(u8, i8)>, // (player_id, direction)
}

impl NetQueue {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        self.inputs.clear();
    }

    pub fn push_input(&mut self, player_id: u8, dir: i8) {
        self.inputs.push((player_id, dir));
    }
}
