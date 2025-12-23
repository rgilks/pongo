/// Game tuning parameters for Pong
#[derive(Debug, Clone, Copy)]
pub struct Params;

impl Params {
    // Arena
    pub const ARENA_WIDTH: f32 = 32.0;
    pub const ARENA_HEIGHT: f32 = 24.0;

    // Paddle
    pub const PADDLE_WIDTH: f32 = 0.8;
    pub const PADDLE_HEIGHT: f32 = 4.0;
    pub const PADDLE_SPEED: f32 = 18.0; // units per second (was 12.0)

    // Ball
    pub const BALL_RADIUS: f32 = 0.5;
    pub const BALL_SPEED_INITIAL: f32 = 12.0; // (was 8.0)
    pub const BALL_SPEED_MAX: f32 = 24.0; // (was 16.0)
    pub const BALL_SPEED_INCREASE: f32 = 1.05; // Multiply speed on paddle hit

    // Score
    pub const WIN_SCORE: u8 = 5; // First to 5 wins

    // Physics
    pub const FIXED_DT: f32 = 0.0166; // ~60 Hz
    pub const MAX_DT: f32 = 0.1; // Clamp to prevent large jumps
}
