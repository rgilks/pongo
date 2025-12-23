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
    pub ball_paddle_overlap: f32, // How much the ball can sink into the paddle
    pub win_score: u8,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            arena_width: 32.0,
            arena_height: 24.0,
            paddle_width: 0.8,   // Match params.rs
            paddle_height: 4.0,
            paddle_speed: 18.0,  // Match params.rs
            ball_radius: 0.5,
            ball_speed_initial: 12.0, // Match params.rs
            ball_speed_max: 24.0,     // Match params.rs
            ball_speed_increase: 1.05,
            ball_paddle_overlap: 0.2, // Match params.rs
            win_score: 5,            // Match params.rs
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
            1.5 // Left paddle
        } else {
            30.5 // Right paddle (arena_width - 1.5)
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

/// Respawn state for managing ball respawn delays after scoring
#[derive(Debug, Clone, Copy, Default)]
pub struct RespawnState {
    pub timer: f32, // Time remaining before ball respawns (0 = ready to respawn)
}

impl RespawnState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn start_delay(&mut self, delay: f32) {
        self.timer = delay;
    }

    pub fn update(&mut self, dt: f32) {
        if self.timer > 0.0 {
            self.timer = (self.timer - dt).max(0.0);
        }
    }

    pub fn can_respawn(&self) -> bool {
        self.timer <= 0.0
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_score_increment_left() {
        let mut score = Score::new();
        assert_eq!(score.left, 0);
        score.increment_left();
        assert_eq!(score.left, 1);
        score.increment_left();
        assert_eq!(score.left, 2);
    }

    #[test]
    fn test_score_increment_right() {
        let mut score = Score::new();
        assert_eq!(score.right, 0);
        score.increment_right();
        assert_eq!(score.right, 1);
        score.increment_right();
        assert_eq!(score.right, 2);
    }

    #[test]
    fn test_score_has_winner_left() {
        let mut score = Score::new();
        for _ in 0..11 {
            score.increment_left();
        }
        assert_eq!(
            score.has_winner(11),
            Some(0),
            "Left player should win at 11"
        );
    }

    #[test]
    fn test_score_has_winner_right() {
        let mut score = Score::new();
        for _ in 0..11 {
            score.increment_right();
        }
        assert_eq!(
            score.has_winner(11),
            Some(1),
            "Right player should win at 11"
        );
    }

    #[test]
    fn test_score_no_winner_below_threshold() {
        let mut score = Score::new();
        for _ in 0..10 {
            score.increment_left();
        }
        assert_eq!(score.has_winner(11), None, "No winner below threshold");
    }

    #[test]
    fn test_events_clear() {
        let mut events = Events::new();
        events.left_scored = true;
        events.right_scored = true;
        events.ball_hit_paddle = true;
        events.ball_hit_wall = true;

        events.clear();

        assert!(!events.left_scored);
        assert!(!events.right_scored);
        assert!(!events.ball_hit_paddle);
        assert!(!events.ball_hit_wall);
    }

    #[test]
    fn test_net_queue_push_input() {
        let mut queue = NetQueue::new();
        queue.push_input(0, -1);
        queue.push_input(1, 1);

        assert_eq!(queue.inputs.len(), 2);
        assert_eq!(queue.inputs[0], (0, -1));
        assert_eq!(queue.inputs[1], (1, 1));
    }

    #[test]
    fn test_net_queue_clear() {
        let mut queue = NetQueue::new();
        queue.push_input(0, -1);
        queue.push_input(1, 1);

        queue.clear();
        assert_eq!(queue.inputs.len(), 0);
    }

    #[test]
    fn test_config_paddle_x() {
        let config = Config::new();
        assert_eq!(config.paddle_x(0), 1.5, "Left paddle X position");
        assert_eq!(config.paddle_x(1), 30.5, "Right paddle X position");
    }

    #[test]
    fn test_config_clamp_paddle_y() {
        let config = Config::new();
        let half_height = config.paddle_height / 2.0;

        // Test clamping below minimum
        assert_eq!(config.clamp_paddle_y(0.0), half_height);

        // Test clamping above maximum
        assert_eq!(
            config.clamp_paddle_y(100.0),
            config.arena_height - half_height
        );

        // Test valid value
        let valid_y = 12.0;
        assert_eq!(config.clamp_paddle_y(valid_y), valid_y);
    }
}
