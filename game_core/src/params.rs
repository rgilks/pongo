/// Game tuning parameters for Pong
#[derive(Debug, Clone, Copy)]
pub struct Params;

impl Params {
    // Arena
    pub const ARENA_WIDTH: f32 = 32.0;
    pub const ARENA_HEIGHT: f32 = 24.0;

    // Paddle
    pub const PADDLE_WIDTH: f32 = 1.0;
    pub const PADDLE_HEIGHT: f32 = 4.0;
    pub const PADDLE_SPEED: f32 = 8.0; // units per second

    // Ball
    pub const BALL_RADIUS: f32 = 0.5;
    pub const BALL_SPEED_INITIAL: f32 = 8.0;
    pub const BALL_SPEED_MAX: f32 = 16.0;
    pub const BALL_SPEED_INCREASE: f32 = 1.05; // Multiply speed on paddle hit

    // Score
    pub const WIN_SCORE: u8 = 11; // First to 11 wins

    // Physics
    pub const FIXED_DT: f32 = 0.0166; // ~60 Hz
    pub const MAX_DT: f32 = 0.1; // Clamp to prevent large jumps
}
