#![allow(unknown_lints)]
#![allow(clippy::manual_is_multiple_of)]
use game_core::*;
use hecs::World;
use js_sys::Date;
use proto::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::time::Duration;
use worker::*;

#[cfg(test)]
mod tests;

// Abstract connection for testing
pub trait GameClient {
    fn send_bytes(&self, bytes: &[u8]) -> Result<()>;
}

impl GameClient for WebSocket {
    fn send_bytes(&self, bytes: &[u8]) -> Result<()> {
        self.send_with_bytes(bytes)
    }
}

// Abstract environment (Time, Logging)
pub trait Environment {
    fn now(&self) -> u64; // ms
    fn log(&self, msg: String);
}

struct WasmEnv;

impl Environment for WasmEnv {
    fn now(&self) -> u64 {
        Date::now() as u64
    }

    fn log(&self, msg: String) {
        // console_log! macro comes from worker crate and takes literal fmt string usually,
        // but we can pass formatted string if we use "%s".
        // Or actually console_log! invokes web_sys::console::log_1.
        console_log!("{}", msg);
    }
}

// Track client activity
pub struct ClientInfo {
    pub client: Box<dyn GameClient>,
    pub last_activity: u64, // Unix timestamp in seconds
}

// Game state wrapper for interior mutability
pub struct GameState {
    pub env: Box<dyn Environment>,
    pub world: World,
    pub time: Time,
    pub map: GameMap,
    pub config: Config,
    pub score: Score,
    pub events: Events,
    pub net_queue: NetQueue,
    pub rng: GameRng,
    pub respawn_state: RespawnState,
    pub clients: HashMap<u8, ClientInfo>, // player_id (0=left, 1=right) -> ClientInfo
    pub next_player_id: u8,
    pub game_started: bool,
    pub tick: u32,
    pub last_input: HashMap<u8, i8>, // Track last input per player to reduce logging
    pub last_tick_time: u64,         // Unix timestamp in ms
    pub accumulator: f32,            // Accumulated time for catch-up steps
}

impl GameState {
    pub fn new(env: Box<dyn Environment>) -> Self {
        let mut world = World::new();
        let map = GameMap::new();
        let config = Config::new();
        let time = Time::default();
        let score = Score::new();
        let events = Events::new();
        let net_queue = NetQueue::new();
        let rng = GameRng::default();

        // Create ball at center
        let ball_pos = map.ball_spawn();
        let ball_vel = glam::Vec2::new(config.ball_speed_initial, 0.0);
        create_ball(&mut world, ball_pos, ball_vel);

        let now = env.now();

        Self {
            env,
            world,
            time,
            map,
            config,
            score,
            events,
            net_queue,
            rng,
            respawn_state: RespawnState::new(),
            clients: HashMap::new(),
            next_player_id: 0,
            game_started: false,
            tick: 0,
            last_input: HashMap::new(),
            last_tick_time: now,
            accumulator: 0.0,
        }
    }

    /// Try to add a player. Returns (player_id, was_empty) if successful.
    pub fn add_player(&mut self, client: Box<dyn GameClient>) -> Option<(u8, bool)> {
        if self.clients.len() >= 2 {
            return None;
        }

        let player_id = self.next_player_id;
        self.next_player_id = (self.next_player_id + 1) % 2;

        let was_empty = self.clients.is_empty();
        let now = self.env.now() / 1000;

        self.clients.insert(
            player_id,
            ClientInfo {
                client,
                last_activity: now,
            },
        );

        // Spawn paddle
        let paddle_y = self.map.paddle_spawn(player_id).y;
        create_paddle(&mut self.world, player_id, paddle_y);

        // Start game if 2 players
        if self.clients.len() == 2 {
            self.game_started = true;
        }

        Some((player_id, was_empty))
    }

    pub fn remove_player(&mut self, player_id: u8) {
        self.clients.remove(&player_id);

        // Despawn paddle
        let entity_to_despawn =
            self.world
                .query::<(&Paddle,)>()
                .iter()
                .find_map(|(entity, (paddle,))| {
                    if paddle.player_id == player_id {
                        Some(entity)
                    } else {
                        None
                    }
                });

        if let Some(entity) = entity_to_despawn {
            let _ = self.world.despawn(entity);
        }

        // Forfeit logic
        if self.game_started {
            if let Some(&remaining_player) = self.clients.keys().next() {
                self.broadcast_game_over(remaining_player);
            }
            self.game_started = false;
        } else if self.clients.len() < 2 {
            self.game_started = false;
        }
    }

    pub fn handle_input(&mut self, player_id: u8, paddle_dir: i8) {
        if let Some(client_info) = self.clients.get_mut(&player_id) {
            let now = self.env.now() / 1000;
            client_info.last_activity = now;

            // Only log when input changes (reduces log spam)
            let last_dir = self.last_input.get(&player_id).copied().unwrap_or(99);
            if paddle_dir != last_dir {
                self.env.log(format!(
                    "DO: Player {player_id} input changed: {last_dir} -> {paddle_dir}"
                ));
                self.last_input.insert(player_id, paddle_dir);
            }

            self.net_queue.push_input(player_id, paddle_dir);
        }
    }

    pub fn step(&mut self) -> Option<u8> {
        if !self.game_started {
            return None;
        }

        self.time.dt = 0.016; // ~60 Hz
        self.tick += 1;

        if self.tick % 60 == 0 {
            self.env.log(format!(
                "DO: Game running, tick={}, clients={}",
                self.tick,
                self.clients.len()
            ));
        }

        game_core::step(
            &mut self.world,
            &mut self.time,
            &self.map,
            &self.config,
            &mut self.score,
            &mut self.events,
            &mut self.net_queue,
            &mut self.rng,
            &mut self.respawn_state,
        );

        // Return winner if any
        if let Some(winner) = self.score.has_winner(self.config.win_score) {
            self.broadcast_game_over(winner);
            self.game_started = false;
            return Some(winner);
        }

        None
    }

    pub fn generate_state_message(&self) -> S2C {
        // Get ball position and velocity
        let (ball_x, ball_y, ball_vx, ball_vy) = self
            .world
            .query::<&Ball>()
            .iter()
            .next()
            .map(|(_e, ball)| (ball.pos.x, ball.pos.y, ball.vel.x, ball.vel.y))
            .unwrap_or((16.0, 12.0, 0.0, 0.0));

        // Get paddle positions
        let mut paddle_left_y = 12.0;
        let mut paddle_right_y = 12.0;
        let mut paddle_count = 0;

        for (_e, paddle) in self.world.query::<&Paddle>().iter() {
            paddle_count += 1;
            if paddle.player_id == 0 {
                paddle_left_y = paddle.y;
            } else if paddle.player_id == 1 {
                paddle_right_y = paddle.y;
            }
        }

        if self.tick % 60 == 0 {
            self.env.log(format!(
                "DO: Paddle state - count={paddle_count}, left_y={paddle_left_y:.1}, right_y={paddle_right_y:.1}"
            ));
        }

        S2C::GameState {
            tick: self.tick,
            ball_x,
            ball_y,
            ball_vx,
            ball_vy,
            paddle_left_y,
            paddle_right_y,
            score_left: self.score.left,
            score_right: self.score.right,
        }
    }

    pub fn broadcast_state(&self) {
        if self.clients.is_empty() {
            return;
        }

        let state_msg = self.generate_state_message();
        if let Ok(bytes) = state_msg.to_bytes() {
            for client_info in self.clients.values() {
                let _ = client_info.client.send_bytes(&bytes);
            }
        }
    }

    pub fn broadcast_game_over(&self, winner: u8) {
        let msg = S2C::GameOver { winner };
        if let Ok(bytes) = msg.to_bytes() {
            for client_info in self.clients.values() {
                let _ = client_info.client.send_bytes(&bytes);
            }
        }
    }
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
        Self {
            state,
            env,
            game_state: RefCell::new(GameState::new(Box::new(WasmEnv))),
        }
    }

    async fn fetch(&self, req: Request) -> Result<Response> {
        console_log!("DO: Received request, method: {:?}", req.method());
        if let Ok(url) = req.url() {
            console_log!("DO: Request URL: {}", url);
        }

        // Check for WebSocket upgrade
        let upgrade_header = req.headers().get("Upgrade");
        console_log!("DO: Upgrade header result: {:?}", upgrade_header);

        match upgrade_header {
            Ok(Some(header)) if header.to_lowercase() == "websocket" => {
                console_log!("DO: Received WebSocket upgrade request");

                let pair = match WebSocketPair::new() {
                    Ok(pair) => pair,
                    Err(err) => {
                        console_error!("DO: Failed to create WebSocket pair: {:?}", err);
                        return Response::error("Failed to create WebSocket pair", 500);
                    }
                };

                let server = pair.server;
                let client = pair.client;

                #[allow(clippy::needless_borrows_for_generic_args)]
                self.state.accept_web_socket(&server);

                console_log!("DO: WebSocket accepted");

                match Response::from_websocket(client) {
                    Ok(resp) => {
                        console_log!("DO: Returning WebSocket 101 response");
                        Ok(resp)
                    }
                    Err(err) => {
                        console_error!("DO: Failed to create WebSocket response: {:?}", err);
                        Response::error("Failed to create WebSocket response", 500)
                    }
                }
            }
            Ok(header_opt) => {
                console_error!("DO: Unexpected Upgrade header state: {:?}", header_opt);
                Response::error("Expected WebSocket upgrade request", 426)
            }
            Err(err) => {
                console_error!("DO: Failed to read Upgrade header: {:?}", err);
                Response::error("Failed to read request headers", 500)
            }
        }
    }

    async fn websocket_message(
        &self,
        ws: WebSocket,
        message: durable::WebSocketIncomingMessage,
    ) -> Result<()> {
        match message {
            durable::WebSocketIncomingMessage::String(_text) => {
                // Ignore text messages
            }
            durable::WebSocketIncomingMessage::Binary(bytes) => match C2S::from_bytes(&bytes) {
                Ok(c2s_msg) => {
                    if let Err(e) = Self::handle_c2s_message(self, ws, c2s_msg).await {
                        console_error!("Error handling C2S message: {e:?}");
                    }
                }
                Err(e) => {
                    console_error!("Failed to parse C2S message: {e:?}");
                }
            },
        }
        Ok(())
    }

    async fn websocket_close(
        &self,
        _ws: WebSocket,
        code: usize,
        reason: String,
        _was_clean: bool,
    ) -> Result<()> {
        console_log!(
            "DO: WebSocket close event (code: {}, reason: {})",
            code,
            reason
        );

        let mut gs = self.game_state.borrow_mut();

        // workaround for missing WS ID
        let player_id_to_remove = gs.clients.keys().next().copied();

        if let Some(player_id) = player_id_to_remove {
            gs.env
                .log(format!("DO: Removing player {player_id} after close event"));
            gs.remove_player(player_id);
        }

        gs.env.log(format!(
            "DO: Remaining clients after cleanup: {}",
            gs.clients.len()
        ));
        Ok(())
    }

    async fn websocket_error(&self, _ws: WebSocket, error: Error) -> Result<()> {
        console_error!("DO: WebSocket error: {:?}", error);
        Ok(())
    }

    #[allow(clippy::await_holding_refcell_ref)] // We drop the RefCell borrow before await
    async fn alarm(&self) -> Result<Response> {
        // Game loop - runs at 60 Hz target
        let tick_interval_ms = 16; // ~60 Hz simulation step

        let mut gs = self.game_state.borrow_mut();

        // Check for idle clients and disconnect them (1 minute timeout)
        let now_ms = gs.env.now();
        let now_seconds = now_ms / 1000;
        let idle_timeout_seconds = 60; // 1 minute
        let mut clients_to_remove = Vec::new();

        for (player_id, client_info) in gs.clients.iter() {
            if now_seconds.saturating_sub(client_info.last_activity) > idle_timeout_seconds {
                gs.env.log(format!(
                    "DO: Client {} idle for {}s, disconnecting",
                    player_id,
                    now_seconds.saturating_sub(client_info.last_activity)
                ));
                clients_to_remove.push(*player_id);
            }
        }

        // Remove idle clients
        for player_id in clients_to_remove {
            gs.remove_player(player_id);
        }

        // Check if we still have clients after cleanup
        let has_clients = !gs.clients.is_empty();
        if !has_clients {
            gs.env
                .log("DO: No clients remaining, stopping alarm loop".to_string());
            drop(gs); // Release borrow before return
            return Response::ok("No clients, stopping alarm loop");
        }

        // Calculate real elapsed time since last alarm
        let elapsed_ms = now_ms.saturating_sub(gs.last_tick_time);
        gs.last_tick_time = now_ms;

        // Add to accumulator, capped to avoid large jumps if DO was hibernated
        gs.accumulator += elapsed_ms.min(100) as f32; // Max 100ms catchup per alarm

        // Run simulation steps
        let mut steps_run = 0;
        const MAX_STEPS: u32 = 10; // Avoid "death spiral" if simulation is too slow

        while gs.accumulator >= tick_interval_ms as f32 && steps_run < MAX_STEPS {
            gs.step();
            gs.accumulator -= tick_interval_ms as f32;
            steps_run += 1;
        }

        if steps_run > 1 && gs.tick % 60 == 0 {
            gs.env.log(format!(
                "DO: Catching up, ran {steps_run} steps in one alarm"
            ));
        }

        // Broadcast state if game is running
        if gs.game_started && (gs.tick == 1 || gs.tick % 3 == 0) {
            gs.broadcast_state();
        }

        // Release borrow before async call
        drop(gs);

        // Schedule next alarm
        self.state
            .storage()
            .set_alarm(Duration::from_millis(tick_interval_ms))
            .await?;

        Response::ok("Alarm processed")
    }
}

impl MatchDO {
    /// Handle incoming C2S message
    async fn handle_c2s_message(&self, ws: WebSocket, msg: C2S) -> Result<()> {
        let should_start_alarm = {
            let mut gs = self.game_state.borrow_mut();
            match msg {
                C2S::Join { code: _, .. } => {
                    // We need to clone WS here because add_player takes ownership
                    if let Some((player_id, was_empty)) = gs.add_player(Box::new(ws.clone())) {
                        gs.env.log(format!(
                            "DO: Player {player_id} joining (clients was empty: {was_empty})"
                        ));
                        // Send Welcome message
                        let welcome = S2C::Welcome { player_id };
                        if let Ok(bytes) = welcome.to_bytes() {
                            let _ = ws.send_with_bytes(&bytes);
                        }

                        // Send initial state
                        let state_msg = gs.generate_state_message();
                        if let Ok(bytes) = state_msg.to_bytes() {
                            // Broadcast to all
                            for client_info in gs.clients.values() {
                                let _ = client_info.client.send_bytes(&bytes);
                            }
                        }
                        Some(was_empty)
                    } else {
                        gs.env
                            .log("DO: Match full, rejecting new player".to_string());
                        None
                    }
                }
                C2S::Input {
                    player_id,
                    paddle_dir,
                    seq: _,
                } => {
                    gs.handle_input(player_id, paddle_dir);
                    None
                }
                C2S::Ping { t_ms } => {
                    // Update activity for all (workaround)
                    let now = gs.env.now() / 1000;
                    for client_info in gs.clients.values_mut() {
                        client_info.last_activity = now;
                    }

                    let pong = S2C::Pong { t_ms };
                    if let Ok(bytes) = pong.to_bytes() {
                        let _ = ws.send_with_bytes(&bytes);
                    }
                    None
                }
            }
        };

        // Start game loop if this was the first player
        if let Some(true) = should_start_alarm {
            self.state
                .storage()
                .set_alarm(Duration::from_millis(16)) // 60 Hz
                .await?;
        }

        Ok(())
    }
}
