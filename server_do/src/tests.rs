use crate::*;
use std::cell::RefCell;
use proto::S2C;

struct MockGameClient {
    pub sent_messages: RefCell<Vec<Vec<u8>>>,
}

impl MockGameClient {
    fn new() -> Self {
        Self {
            sent_messages: RefCell::new(Vec::new()),
        }
    }
    
    // Kept to avoid unused warnings/tests
    #[allow(dead_code)]
    fn last_message(&self) -> Option<Vec<u8>> {
        self.sent_messages.borrow().last().cloned()
    }
    
    #[allow(dead_code)]
    fn count(&self) -> usize {
        self.sent_messages.borrow().len()
    }
    
    #[allow(dead_code)]
    fn clear(&self) {
        self.sent_messages.borrow_mut().clear();
    }
}

impl GameClient for MockGameClient {
    fn send_bytes(&self, bytes: &[u8]) -> Result<()> {
        self.sent_messages.borrow_mut().push(bytes.to_vec());
        Ok(())
    }
}

struct MockEnv {
    time_ms: u64,
}

impl MockEnv {
    fn new() -> Self {
        Self { time_ms: 1000 }
    }
}

impl Environment for MockEnv {
    fn now(&self) -> u64 {
        self.time_ms
    }
    fn log(&self, _msg: String) {
        // No-op for tests or println!(_msg)
    }
}

#[test]
fn test_game_initialization() {
    let gs = GameState::new(Box::new(MockEnv::new()));
    assert_eq!(gs.clients.len(), 0);
    assert_eq!(gs.next_player_id, 0);
    assert!(!gs.game_started);
}

#[test]
fn test_add_player_limit() {
    let mut gs = GameState::new(Box::new(MockEnv::new()));
    
    // Add player 0
    let res0 = gs.add_player(Box::new(MockGameClient::new()));
    assert!(res0.is_some());
    let (pid0, empty0) = res0.unwrap();
    assert_eq!(pid0, 0);
    assert!(empty0); // Was empty
    
    // Add player 1
    let res1 = gs.add_player(Box::new(MockGameClient::new()));
    assert!(res1.is_some());
    let (pid1, empty1) = res1.unwrap();
    assert_eq!(pid1, 1);
    assert!(!empty1); // Was not empty
    
    // Add player 2 (should fail)
    let res2 = gs.add_player(Box::new(MockGameClient::new()));
    assert!(res2.is_none());
}

#[test]
fn test_game_start_condition() {
    let mut gs = GameState::new(Box::new(MockEnv::new()));
    
    gs.add_player(Box::new(MockGameClient::new()));
    assert!(!gs.game_started);
    
    gs.add_player(Box::new(MockGameClient::new()));
    assert!(gs.game_started);
}

#[test]
fn test_player_removal() {
    let mut gs = GameState::new(Box::new(MockEnv::new()));
    
    gs.add_player(Box::new(MockGameClient::new()));
    gs.remove_player(0);
    
    assert_eq!(gs.clients.len(), 0);
}

#[test]
fn test_handle_input() {
    let mut gs = GameState::new(Box::new(MockEnv::new()));
    let client0 = Box::new(MockGameClient::new());
    gs.add_player(client0);
    
    // Send input for player 0
    gs.handle_input(0, 1); // Move down
    
    // Check if input queue has it
    let inputs = gs.net_queue.pop_inputs();
    assert!(!inputs.is_empty());
    assert_eq!(inputs[0].0, 0);
    assert_eq!(inputs[0].1, 1);
}

#[test]
fn test_broadcast_state() {
    let mut gs = GameState::new(Box::new(MockEnv::new()));
    
    let messages = std::rc::Rc::new(RefCell::new(Vec::new()));
    
    struct SharedMock {
        msgs: std::rc::Rc<RefCell<Vec<Vec<u8>>>>,
    }
    
    impl GameClient for SharedMock {
        fn send_bytes(&self, bytes: &[u8]) -> Result<()> {
            self.msgs.borrow_mut().push(bytes.to_vec());
            Ok(())
        }
    }
    
    let client = Box::new(SharedMock { msgs: messages.clone() });
    
    gs.add_player(client);
    
    gs.broadcast_state();
    
    assert_eq!(messages.borrow().len(), 1);
    
    // Verify it's a GameState message
    let bytes = &messages.borrow()[0];
    let msg = S2C::from_bytes(bytes).unwrap();
    match msg {
        S2C::GameState { .. } => assert!(true),
        _ => assert!(false, "Expected GameState message"),
    }
}
