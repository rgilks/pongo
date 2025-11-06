//! Network protocol for ISO game
//!
//! Uses postcard for serialization with quantization for efficient transfer

use postcard::{from_bytes, to_allocvec};

// Quantization constants
pub const WORLD_BOUNDS: f32 = 32.0;
pub const POS_SCALE: f32 = 1024.0; // i16 → ±32u with 1024 steps/u
pub const YAW_SCALE: f32 = 65535.0 / (2.0 * std::f32::consts::PI); // u16 → 0..2π
pub const ENERGY_SCALE: f32 = 10.0; // u16 (0..1000) → 0..100.0

// ============================================================================
// C2S Messages (Client to Server)
// ============================================================================

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum C2S {
    Join {
        code: [u8; 5],
        avatar: u8,
        name_id: u8,
    },
    Input {
        seq: u32,
        t_ms: u32,
        thrust_i8: i8,
        turn_i8: i8,
        bolt: u8,   // 0..3
        shield: u8, // 0..3
    },
    Ping {
        t_ms: u32,
    },
    Ack {
        snapshot_id: u32,
    },
}

// ============================================================================
// S2C Messages (Server to Client)
// ============================================================================

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum S2C {
    Welcome {
        player_id: u16,
        params_hash: u32,
        map_rev: u16,
    },
    Snapshot {
        id: u32,
        tick: u32,
        t_ms: u32,
        last_seq_ack: u32,
        players: Vec<PlayerP>,
        bolts: Vec<BoltP>,
        pickups: Vec<PickupP>,
        hill_owner: Option<u16>,
        hill_progress_u16: u16,
    },
    Eliminated {
        player_id: u16,
    },
    Ended {
        standings: Vec<(u16, u16)>, // (player_id, points)
    },
}

// ============================================================================
// Quantized Data Types
// ============================================================================

/// Quantized player data
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PlayerP {
    pub id: u16,
    pub pos_q: [i16; 2],
    pub vel_q: [i16; 2],
    pub yaw_q: u16,
    pub bolt_max: u8,
    pub shield_max: u8,
    pub hp: u8,
    pub energy_q: u16,
    pub flags: u8,
}

/// Quantized bolt data
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BoltP {
    pub id: u16,
    pub pos_q: [i16; 2],
    pub vel_q: [i16; 2],
    pub rad_q: u8,
    pub level: u8,
    pub owner: u16,
}

/// Quantized pickup data
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PickupP {
    pub id: u16,
    pub pos_q: [i16; 2],
    pub kind: u8, // 0=Health, 1=BoltUp, 2=ShieldMod
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

// ============================================================================
// Quantization Helpers
// ============================================================================

/// Quantize position to i16
pub fn quantize_pos(pos: f32) -> i16 {
    (pos * POS_SCALE).clamp(-32768.0, 32767.0) as i16
}

/// Dequantize position from i16
pub fn dequantize_pos(pos_q: i16) -> f32 {
    pos_q as f32 / POS_SCALE
}

/// Quantize yaw to u16 (0..65535 → 0..2π)
pub fn quantize_yaw(yaw: f32) -> u16 {
    let normalized = (yaw % (2.0 * std::f32::consts::PI) + 2.0 * std::f32::consts::PI)
        % (2.0 * std::f32::consts::PI);
    (normalized * YAW_SCALE) as u16
}

/// Dequantize yaw from u16
pub fn dequantize_yaw(yaw_q: u16) -> f32 {
    yaw_q as f32 / YAW_SCALE
}

/// Quantize energy to u16 (0..1000 → 0..100.0)
pub fn quantize_energy(energy: f32) -> u16 {
    (energy * ENERGY_SCALE).clamp(0.0, 1000.0) as u16
}

/// Dequantize energy from u16
pub fn dequantize_energy(energy_q: u16) -> f32 {
    energy_q as f32 / ENERGY_SCALE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_c2s_serialization() {
        let msg = C2S::Input {
            seq: 1,
            t_ms: 1000,
            thrust_i8: 100,
            turn_i8: -50,
            bolt: 1,
            shield: 2,
        };
        let bytes = msg.to_bytes().unwrap();
        let decoded = C2S::from_bytes(&bytes).unwrap();
        match (msg, decoded) {
            (
                C2S::Input {
                    seq: s1,
                    t_ms: t1,
                    thrust_i8: th1,
                    turn_i8: tu1,
                    bolt: b1,
                    shield: sh1,
                },
                C2S::Input {
                    seq: s2,
                    t_ms: t2,
                    thrust_i8: th2,
                    turn_i8: tu2,
                    bolt: b2,
                    shield: sh2,
                },
            ) => {
                assert_eq!(s1, s2);
                assert_eq!(t1, t2);
                assert_eq!(th1, th2);
                assert_eq!(tu1, tu2);
                assert_eq!(b1, b2);
                assert_eq!(sh1, sh2);
            }
            _ => panic!("Message type mismatch"),
        }
    }

    #[test]
    fn test_quantization() {
        let pos = 10.5;
        let pos_q = quantize_pos(pos);
        let pos_deq = dequantize_pos(pos_q);
        assert!((pos - pos_deq).abs() < 0.001);

        let yaw = 1.5;
        let yaw_q = quantize_yaw(yaw);
        let yaw_deq = dequantize_yaw(yaw_q);
        assert!((yaw - yaw_deq).abs() < 0.01);

        let energy = 75.5;
        let energy_q = quantize_energy(energy);
        let energy_deq = dequantize_energy(energy_q);
        assert!((energy - energy_deq).abs() < 0.1);
    }
}
