//! Game State Machine
//!
//! Manages game state transitions for both local and multiplayer modes.

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

/// Game states
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FsmState {
    Idle,
    CountdownLocal,
    PlayingLocal,
    Connecting,
    Waiting,
    CountdownMulti,
    PlayingMulti,
    GameOverLocal,
    GameOverMulti,
    Disconnected,
}

/// Actions that trigger state transitions
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameAction {
    StartLocal,
    CreateMatch,
    JoinMatch,
    CountdownDone,
    Quit,
    GameOver,
    Connected,
    ConnectionFailed,
    OpponentJoined,
    Disconnected,
    Leave,
    PlayAgain,
    RematchStarted,
}

/// Result of a state transition
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
#[derive(Debug, Clone)]
pub struct TransitionResult {
    success: bool,
    from_state: FsmState,
    to_state: FsmState,
    action: GameAction,
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
impl TransitionResult {
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen(getter))]
    pub fn success(&self) -> bool {
        self.success
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen(getter))]
    #[allow(clippy::wrong_self_convention)]
    pub fn from_state(&self) -> FsmState {
        self.from_state
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen(getter))]
    pub fn to_state(&self) -> FsmState {
        self.to_state
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen(getter))]
    pub fn action(&self) -> GameAction {
        self.action
    }
}

/// Game Finite State Machine
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub struct GameFsm {
    state: FsmState,
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
impl GameFsm {
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen(constructor))]
    pub fn new() -> Self {
        Self {
            state: FsmState::Idle,
        }
    }

    /// Get current state
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen(getter))]
    pub fn state(&self) -> FsmState {
        self.state
    }

    /// Get current state as string (for JS interop)
    pub fn state_string(&self) -> String {
        format!("{:?}", self.state)
    }

    /// Check if a transition is valid
    pub fn can_transition(&self, action: GameAction) -> bool {
        self.get_next_state(action).is_some()
    }

    /// Attempt a transition
    pub fn transition(&mut self, action: GameAction) -> TransitionResult {
        let from_state = self.state;

        if let Some(next_state) = self.get_next_state(action) {
            self.state = next_state;
            TransitionResult {
                success: true,
                from_state,
                to_state: next_state,
                action,
            }
        } else {
            TransitionResult {
                success: false,
                from_state,
                to_state: from_state,
                action,
            }
        }
    }

    /// Transition using action string (for easier JS interop)
    pub fn transition_str(&mut self, action: &str) -> TransitionResult {
        let action = match action {
            "START_LOCAL" => GameAction::StartLocal,
            "CREATE_MATCH" => GameAction::CreateMatch,
            "JOIN_MATCH" => GameAction::JoinMatch,
            "COUNTDOWN_DONE" => GameAction::CountdownDone,
            "QUIT" => GameAction::Quit,
            "GAME_OVER" => GameAction::GameOver,
            "CONNECTED" => GameAction::Connected,
            "CONNECTION_FAILED" => GameAction::ConnectionFailed,
            "OPPONENT_JOINED" => GameAction::OpponentJoined,
            "DISCONNECTED" => GameAction::Disconnected,
            "LEAVE" => GameAction::Leave,
            "PLAY_AGAIN" => GameAction::PlayAgain,
            "REMATCH_STARTED" => GameAction::RematchStarted,
            _ => {
                return TransitionResult {
                    success: false,
                    from_state: self.state,
                    to_state: self.state,
                    action: GameAction::Leave, // Default, won't be used
                };
            }
        };
        self.transition(action)
    }

    /// Get next state for a given action (if valid)
    fn get_next_state(&self, action: GameAction) -> Option<FsmState> {
        match (self.state, action) {
            // From Idle
            (FsmState::Idle, GameAction::StartLocal) => Some(FsmState::CountdownLocal),
            (FsmState::Idle, GameAction::CreateMatch) => Some(FsmState::Connecting),
            (FsmState::Idle, GameAction::JoinMatch) => Some(FsmState::Connecting),

            // From CountdownLocal
            (FsmState::CountdownLocal, GameAction::CountdownDone) => Some(FsmState::PlayingLocal),
            (FsmState::CountdownLocal, GameAction::Quit) => Some(FsmState::Idle),

            // From PlayingLocal
            (FsmState::PlayingLocal, GameAction::GameOver) => Some(FsmState::GameOverLocal),
            (FsmState::PlayingLocal, GameAction::Quit) => Some(FsmState::Idle),

            // From Connecting
            (FsmState::Connecting, GameAction::Connected) => Some(FsmState::Waiting),
            (FsmState::Connecting, GameAction::ConnectionFailed) => Some(FsmState::Idle),

            // From Waiting
            (FsmState::Waiting, GameAction::OpponentJoined) => Some(FsmState::CountdownMulti),
            (FsmState::Waiting, GameAction::Disconnected) => Some(FsmState::Idle),
            (FsmState::Waiting, GameAction::Leave) => Some(FsmState::Idle),

            // From CountdownMulti
            (FsmState::CountdownMulti, GameAction::CountdownDone) => Some(FsmState::PlayingMulti),
            (FsmState::CountdownMulti, GameAction::Disconnected) => Some(FsmState::Disconnected),

            // From PlayingMulti
            (FsmState::PlayingMulti, GameAction::GameOver) => Some(FsmState::GameOverMulti),
            (FsmState::PlayingMulti, GameAction::Disconnected) => Some(FsmState::Disconnected),

            // From GameOverLocal
            (FsmState::GameOverLocal, GameAction::PlayAgain) => Some(FsmState::CountdownLocal),
            (FsmState::GameOverLocal, GameAction::Leave) => Some(FsmState::Idle),

            // From GameOverMulti
            (FsmState::GameOverMulti, GameAction::RematchStarted) => Some(FsmState::CountdownMulti),
            (FsmState::GameOverMulti, GameAction::Disconnected) => Some(FsmState::Disconnected),
            (FsmState::GameOverMulti, GameAction::Leave) => Some(FsmState::Idle),

            // From Disconnected
            (FsmState::Disconnected, GameAction::Leave) => Some(FsmState::Idle),

            // Invalid transition
            _ => None,
        }
    }

    /// Reset to Idle state
    pub fn reset(&mut self) {
        self.state = FsmState::Idle;
    }

    /// Check if currently in a playing state
    pub fn is_playing(&self) -> bool {
        matches!(self.state, FsmState::PlayingLocal | FsmState::PlayingMulti)
    }

    /// Check if in a multiplayer state
    pub fn is_multiplayer(&self) -> bool {
        matches!(
            self.state,
            FsmState::Connecting
                | FsmState::Waiting
                | FsmState::CountdownMulti
                | FsmState::PlayingMulti
                | FsmState::GameOverMulti
        )
    }

    /// Check if in game over state
    pub fn is_game_over(&self) -> bool {
        matches!(
            self.state,
            FsmState::GameOverLocal | FsmState::GameOverMulti
        )
    }
}

impl Default for GameFsm {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_state() {
        let fsm = GameFsm::new();
        assert_eq!(fsm.state(), FsmState::Idle);
    }

    #[test]
    fn test_valid_transition() {
        let mut fsm = GameFsm::new();
        let result = fsm.transition(GameAction::StartLocal);
        assert!(result.success);
        assert_eq!(fsm.state(), FsmState::CountdownLocal);
    }

    #[test]
    fn test_invalid_transition() {
        let mut fsm = GameFsm::new();
        let result = fsm.transition(GameAction::GameOver);
        assert!(!result.success);
        assert_eq!(fsm.state(), FsmState::Idle);
    }

    #[test]
    fn test_local_game_flow() {
        let mut fsm = GameFsm::new();
        fsm.transition(GameAction::StartLocal);
        fsm.transition(GameAction::CountdownDone);
        assert_eq!(fsm.state(), FsmState::PlayingLocal);
        fsm.transition(GameAction::GameOver);
        assert_eq!(fsm.state(), FsmState::GameOverLocal);
        fsm.transition(GameAction::PlayAgain);
        assert_eq!(fsm.state(), FsmState::CountdownLocal);
    }

    #[test]
    fn test_multiplayer_flow() {
        let mut fsm = GameFsm::new();
        fsm.transition(GameAction::CreateMatch);
        assert_eq!(fsm.state(), FsmState::Connecting);
        fsm.transition(GameAction::Connected);
        assert_eq!(fsm.state(), FsmState::Waiting);
        fsm.transition(GameAction::OpponentJoined);
        assert_eq!(fsm.state(), FsmState::CountdownMulti);
        fsm.transition(GameAction::CountdownDone);
        assert_eq!(fsm.state(), FsmState::PlayingMulti);
    }

    #[test]
    fn test_transition_str() {
        let mut fsm = GameFsm::new();
        let result = fsm.transition_str("START_LOCAL");
        assert!(result.success);
        assert_eq!(fsm.state(), FsmState::CountdownLocal);
    }
}
