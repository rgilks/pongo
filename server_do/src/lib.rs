use game_core::*;
use hecs::World;
use std::cell::RefCell;
use std::collections::HashMap;
use worker::*;

// Game state wrapper for interior mutability
struct GameState {
    world: World,
    time: Time,
    map: GameMap,
    rng: GameRng,
    score: Score,
    events: Events,
    config: Config,
    net_queue: NetQueue,
    #[allow(dead_code)]
    clients: HashMap<u16, WebSocket>,
    #[allow(dead_code)]
    next_player_id: u16,
    #[allow(dead_code)]
    match_started: bool,
    #[allow(dead_code)]
    last_tick_ms: u64,
}

#[durable_object]
pub struct MatchDO {
    state: State,
    #[allow(dead_code)]
    env: Env,
    game_state: RefCell<GameState>,
}

impl DurableObject for MatchDO {
    fn new(state: State, env: Env) -> Self {
        // Initialize game state
        let world = World::new();
        let time = Time::new();
        let map = GameMap::new(Map::test_map());
        let rng = GameRng::new();
        let score = Score::new();
        let events = Events::new();
        let config = Config::new();
        let net_queue = NetQueue::new();

        let game_state = GameState {
            world,
            time,
            map,
            rng,
            score,
            events,
            config,
            net_queue,
            clients: HashMap::new(),
            next_player_id: 1,
            match_started: false,
            last_tick_ms: 0,
        };

        Self {
            state,
            env,
            game_state: RefCell::new(game_state),
        }
    }

    async fn fetch(&self, req: Request) -> Result<Response> {
        // Check if this is a WebSocket upgrade request
        let upgrade_header = req.headers().get("Upgrade")?;

        if upgrade_header == Some("websocket".to_string()) {
            // Create WebSocket pair
            let pair = WebSocketPair::new()?;
            let server = pair.server;
            let client = pair.client;

            // Accept the WebSocket connection
            #[allow(clippy::needless_borrows_for_generic_args)]
            self.state.accept_web_socket(&server);

            // TODO: Assign player_id and send Welcome message
            // For now, just send a simple welcome
            server.send_with_str("Welcome to ISO Match!")?;

            // Return response with WebSocket
            Ok(Response::from_websocket(client)?)
        } else {
            Response::ok("Match Durable Object - Connect via WebSocket")
        }
    }

    async fn websocket_message(
        &self,
        ws: WebSocket,
        message: durable::WebSocketIncomingMessage,
    ) -> Result<()> {
        // Handle incoming WebSocket messages
        match message {
            durable::WebSocketIncomingMessage::String(text) => {
                // For now, echo back (will implement proper protocol later)
                ws.send_with_str(format!("Echo: {}", text))?;
            }
            durable::WebSocketIncomingMessage::Binary(bytes) => {
                // Binary message - will handle C2S protocol here
                // TODO: Parse Input message from proto and add to net_queue
                // For now, trigger a simulation step
                Self::step_game_simulation(&mut self.game_state.borrow_mut());
                ws.send_with_bytes(&bytes)?;
            }
        }
        Ok(())
    }

    async fn websocket_close(
        &self,
        _ws: WebSocket,
        _code: usize,
        _reason: String,
        _was_clean: bool,
    ) -> Result<()> {
        // Remove client from tracking
        // TODO: Find player_id for this WebSocket and remove
        // For now, just acknowledge
        Ok(())
    }

    async fn websocket_error(&self, _ws: WebSocket, _error: Error) -> Result<()> {
        // Handle WebSocket errors
        // TODO: Log error and remove client
        Ok(())
    }
}

impl MatchDO {
    /// Step the game simulation
    fn step_game_simulation(gs: &mut GameState) {
        // Update time
        gs.time.dt = 0.05; // 50ms step

        // Run game simulation step
        step(
            &mut gs.world,
            &mut gs.time,
            &gs.map,
            &mut gs.rng,
            &mut gs.score,
            &mut gs.events,
            &gs.config,
            &mut gs.net_queue,
        );

        // TODO: Broadcast snapshot to all connected clients
        // For now, just clear the net queue
        gs.net_queue.inputs.clear();
        gs.net_queue.acks.clear();
    }
}
