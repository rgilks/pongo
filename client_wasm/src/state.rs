//! Game state management with interpolation

pub use proto::GameStateSnapshot;

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
    pub my_player_id: Option<u8>,
    pub winner: Option<u8>,
    // Smooth correction state for ball position
    ball_display_x: f32,
    ball_display_y: f32,
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
            score_left: 0,
            score_right: 0,
        };
        Self {
            current: initial.clone(),
            previous: initial,
            interpolation_alpha: 1.0,
            time_since_update: 0.0,
            score_left: 0,
            score_right: 0,
            my_player_id: None,
            winner: None,
            ball_display_x: 16.0,
            ball_display_y: 12.0,
        }
    }

    /// Update interpolation based on elapsed time
    /// Target: 60fps render, 20-60Hz server updates
    pub fn update_interpolation(&mut self, dt: f32) {
        self.time_since_update += dt;
        // Server sends updates at 20Hz (50ms). Use 100ms (2x) for jitter tolerance.
        let interpolation_duration = 0.100;
        self.interpolation_alpha = (self.time_since_update / interpolation_duration).min(1.0);

        // Smoothly blend display position toward target using exponential smoothing
        // This prevents jarring jumps when new server state arrives
        let target_x = self.extrapolate_ball_internal(self.current.ball_x, self.current.ball_vx);
        let target_y = self.extrapolate_ball_internal(self.current.ball_y, self.current.ball_vy);

        // Smoothing factor: higher = faster convergence (0.3 = ~3 frames to 90% convergence)
        let smoothing = 0.3;
        self.ball_display_x += (target_x - self.ball_display_x) * smoothing;
        self.ball_display_y += (target_y - self.ball_display_y) * smoothing;
    }

    /// Get interpolated position with basic lerp
    fn interpolate(&self, prev: f32, curr: f32) -> f32 {
        prev + (curr - prev) * self.interpolation_alpha
    }

    /// Internal extrapolation with clamped time to prevent overshooting
    fn extrapolate_ball_internal(&self, pos: f32, vel: f32) -> f32 {
        // Clamp extrapolation to max 100ms to prevent large jumps on network delays
        let clamped_time = self.time_since_update.min(0.100);
        pos + vel * clamped_time
    }

    /// Get current ball X with smooth display position
    pub fn get_ball_x(&self) -> f32 {
        self.ball_display_x
    }

    /// Get current ball Y with smooth display position
    pub fn get_ball_y(&self) -> f32 {
        self.ball_display_y
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
        // Note: ball_display_x/y are NOT reset here - they smoothly converge via update_interpolation
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

    pub fn set_winner(&mut self, winner: u8) {
        self.winner = Some(winner);
    }

    pub fn time_since_update(&self) -> f32 {
        self.time_since_update
    }

    pub fn get_current_snapshot(&self) -> Option<GameStateSnapshot> {
        Some(self.current.clone())
    }
}
