//! Network message handling

use crate::state::GameState;
use proto::{C2S, S2C};

/// Handle incoming server message
pub fn handle_message(msg: S2C, game_state: &mut GameState) -> Result<(), String> {
    match msg {
        S2C::Welcome { player_id } => {
            game_state.set_player_id(player_id);
        }
        S2C::GameState {
            ball_x,
            ball_y,
            paddle_left_y,
            paddle_right_y,
            score_left,
            score_right,
            ball_vx,
            ball_vy,
            tick,
        } => {
            use crate::state::GameStateSnapshot;
            game_state.set_current(GameStateSnapshot {
                ball_x,
                ball_y,
                paddle_left_y,
                paddle_right_y,
                ball_vx,
                ball_vy,
                tick,
            });
            game_state.set_scores(score_left, score_right);
        }
        S2C::GameOver { winner: _ } => {
            // Game over - winner determined
            // Could update UI here if needed
        }
        S2C::Pong { t_ms: _ } => {
            // Ping response handled by caller, should not reach here
            return Err("Pong message should be handled separately".to_string());
        }
    }
    Ok(())
}

/// Create join message bytes
pub fn create_join_message(code: &str) -> Result<Vec<u8>, String> {
    let code_bytes: Vec<u8> = code.bytes().take(5).collect();
    if code_bytes.len() != 5 {
        return Err("Match code must be exactly 5 characters".to_string());
    }
    let mut code_array = [0u8; 5];
    code_array.copy_from_slice(&code_bytes[..5]);
    C2S::Join { code: code_array }
        .to_bytes()
        .map_err(|e| format!("Failed to serialize join message: {:?}", e))
}

/// Create input message bytes
pub fn create_input_message(player_id: u8, paddle_dir: i8, seq: u32) -> Result<Vec<u8>, String> {
    C2S::Input {
        player_id,
        paddle_dir,
        seq,
    }
    .to_bytes()
    .map_err(|e| format!("Failed to serialize input message: {:?}", e))
}

/// Create ping message bytes
pub fn create_ping_message(t_ms: u32) -> Result<Vec<u8>, String> {
    C2S::Ping { t_ms }
        .to_bytes()
        .map_err(|e| format!("Failed to serialize ping message: {:?}", e))
}
