//! Network protocol for Pong game
//!
//! Uses postcard for efficient binary serialization

use postcard::{from_bytes, to_allocvec};

// ============================================================================
// C2S Messages (Client to Server)
// ============================================================================

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum C2S {
    /// Join a match with code
    Join { code: [u8; 5] },

    /// Paddle input: -1 = up, 0 = stop, 1 = down
    /// seq: Client-side sequence number for input prediction (not used by server, but included for future use)
    Input {
        player_id: u8,
        paddle_dir: i8,
        seq: u32,
    },

    /// Ping for latency measurement
    Ping { t_ms: u32 },
}

// ============================================================================
// S2C Messages (Server to Client)
// ============================================================================

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum S2C {
    /// Welcome message with player assignment
    Welcome {
        player_id: u8, // 0 = left, 1 = right
    },

    /// Game state snapshot
    GameState {
        tick: u32,
        ball_x: f32,
        ball_y: f32,
        ball_vx: f32,
        ball_vy: f32,
        paddle_left_y: f32,
        paddle_right_y: f32,
        score_left: u8,
        score_right: u8,
    },

    /// Game over message
    GameOver {
        winner: u8, // 0 = left, 1 = right
    },

    /// Pong response to ping
    Pong { t_ms: u32 },
}

// ============================================================================
// Serialization Helpers
// ============================================================================

impl C2S {
    /// Serialize C2S message to bytes
    pub fn to_bytes(&self) -> Result<Vec<u8>, postcard::Error> {
        to_allocvec(self)
    }

    /// Deserialize C2S message from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, postcard::Error> {
        from_bytes(bytes)
    }
}

impl S2C {
    /// Serialize S2C message to bytes
    pub fn to_bytes(&self) -> Result<Vec<u8>, postcard::Error> {
        to_allocvec(self)
    }

    /// Deserialize S2C message from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, postcard::Error> {
        from_bytes(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_c2s_serialization() {
        let msg = C2S::Input {
            player_id: 0,
            paddle_dir: -1,
            seq: 1,
        };
        let bytes = msg.to_bytes().expect("Serialization should succeed");
        let decoded = C2S::from_bytes(&bytes).expect("Deserialization should succeed");
        match (msg, decoded) {
            (
                C2S::Input {
                    player_id: p1,
                    paddle_dir: d1,
                    seq: s1,
                },
                C2S::Input {
                    player_id: p2,
                    paddle_dir: d2,
                    seq: s2,
                },
            ) => {
                assert_eq!(p1, p2);
                assert_eq!(d1, d2);
                assert_eq!(s1, s2);
            }
            _ => panic!("Message type mismatch"),
        }
    }

    #[test]
    fn test_s2c_serialization() {
        let msg = S2C::GameState {
            tick: 100,
            ball_x: 16.0,
            ball_y: 12.0,
            ball_vx: 8.0,
            ball_vy: 4.0,
            paddle_left_y: 12.0,
            paddle_right_y: 12.0,
            score_left: 5,
            score_right: 3,
        };
        let bytes = msg.to_bytes().expect("Serialization should succeed");
        let decoded = S2C::from_bytes(&bytes).expect("Deserialization should succeed");
        match decoded {
            S2C::GameState { tick, ball_x, .. } => {
                assert_eq!(tick, 100);
                assert_eq!(ball_x, 16.0);
            }
            _ => panic!("Message type mismatch"),
        }
    }
}
