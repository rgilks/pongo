//! Game state management with interpolation

/// Previous and current game state for interpolation
#[derive(Clone, Debug)]
pub struct GameStateSnapshot {
    pub ball_x: f32,
    pub ball_y: f32,
    pub paddle_left_y: f32,
    pub paddle_right_y: f32,
    pub ball_vx: f32,
    pub ball_vy: f32,
    pub tick: u32,
}

/// Game state tracking with interpolation
pub struct GameState {
    // Current authoritative state from server
    current: GameStateSnapshot,
    // Previous state for interpolation
    previous: GameStateSnapshot,
    // Interpolation time (0.0 = previous, 1.0 = current)
    interpolation_alpha: f32,
    // Time since last state update
    time_since_update: f32,
    // Score (doesn't need interpolation)
    score_left: u8,
    score_right: u8,
    my_player_id: Option<u8>,
}

impl GameState {
    pub fn new() -> Self {
        let initial = GameStateSnapshot {
            ball_x: 16.0,
            ball_y: 12.0,
            paddle_left_y: 12.0,
            paddle_right_y: 12.0,
            ball_vx: 0.0,
            ball_vy: 0.0,
            tick: 0,
        };
        Self {
            current: initial.clone(),
            previous: initial,
            interpolation_alpha: 1.0,
            time_since_update: 0.0,
            score_left: 0,
            score_right: 0,
            my_player_id: None,
        }
    }

    /// Update interpolation based on elapsed time
    /// Target: 60fps render, 20-60Hz server updates
    pub fn update_interpolation(&mut self, dt: f32) {
        self.time_since_update += dt;
        // Interpolate over ~60ms (20Hz update rate = 1/20 = 0.05s)
        // Use slightly longer duration to handle jitter and ensure smoothness
        let interpolation_duration = 0.06; // 60ms for smoother interpolation
        self.interpolation_alpha = (self.time_since_update / interpolation_duration).min(1.0);
    }

    /// Get interpolated position
    fn interpolate(&self, prev: f32, curr: f32) -> f32 {
        prev + (curr - prev) * self.interpolation_alpha
    }

    /// Get current interpolated positions
    pub fn get_ball_x(&self) -> f32 {
        self.interpolate(self.previous.ball_x, self.current.ball_x)
    }

    pub fn get_ball_y(&self) -> f32 {
        self.interpolate(self.previous.ball_y, self.current.ball_y)
    }

    pub fn get_paddle_left_y(&self) -> f32 {
        self.interpolate(self.previous.paddle_left_y, self.current.paddle_left_y)
    }

    pub fn get_paddle_right_y(&self) -> f32 {
        self.interpolate(self.previous.paddle_right_y, self.current.paddle_right_y)
    }

    pub fn set_current(&mut self, snapshot: GameStateSnapshot) {
        self.previous = self.current.clone();
        self.current = snapshot;
        self.time_since_update = 0.0;
        self.interpolation_alpha = 0.0;
    }

    pub fn set_scores(&mut self, left: u8, right: u8) {
        self.score_left = left;
        self.score_right = right;
    }

    pub fn get_scores(&self) -> (u8, u8) {
        (self.score_left, self.score_right)
    }

    pub fn set_player_id(&mut self, player_id: u8) {
        self.my_player_id = Some(player_id);
    }

    pub fn get_player_id(&self) -> Option<u8> {
        self.my_player_id
    }

    pub fn time_since_update(&self) -> f32 {
        self.time_since_update
    }
}
