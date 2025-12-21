use game_core::*;
use hecs::World;
use js_sys::Date;
use proto::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::time::Duration;
use worker::*;

// Track client activity
struct ClientInfo {
    ws: WebSocket,
    last_activity: u64, // Unix timestamp in seconds
}

// Game state wrapper for interior mutability
struct GameState {
    world: World,
    time: Time,
    map: GameMap,
    config: Config,
    score: Score,
    events: Events,
    net_queue: NetQueue,
    rng: GameRng,
    respawn_state: RespawnState,
    clients: HashMap<u8, ClientInfo>, // player_id (0=left, 1=right) -> ClientInfo
    next_player_id: u8,
    game_started: bool,
    tick: u32,
    last_input: HashMap<u8, i8>, // Track last input per player to reduce logging
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
        // Initialize Pong game state
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

        let game_state = GameState {
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
        };

        Self {
            state,
            env,
            game_state: RefCell::new(game_state),
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

        // Find and remove the client that closed
        // LIMITATION: In Cloudflare Workers, we can't directly compare WebSocket instances,
        // so we can't identify which specific client closed. The close event is called for
        // the specific WebSocket that closed, but we can't match it to our stored clients.
        //
        // Current workaround: Remove the first client. This is imperfect but acceptable
        // since we only support 2 players max, and the idle timeout will clean up
        // disconnected clients anyway. In practice, this rarely causes issues because:
        // 1. We only have 2 players max
        // 2. The idle timeout (60s) will remove truly disconnected clients
        // 3. If a client disconnects, the game stops anyway (needs 2 players)
        //
        // A better solution would require WebSocket IDs or a way to compare WebSocket instances,
        // which the Cloudflare Workers API doesn't currently provide.
        let player_id_to_remove = gs.clients.keys().next().copied();

        if let Some(player_id) = player_id_to_remove {
            console_log!("DO: Removing player {} after close event", player_id);
            gs.clients.remove(&player_id);

            // Despawn paddle
            let entity_to_despawn =
                gs.world
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
                let _ = gs.world.despawn(entity);
                console_log!("DO: Despawned paddle for player {}", player_id);
            }

            // If game was running and a player remains, declare them winner (Forfeit)
            if gs.game_started {
                if let Some(&remaining_player) = gs.clients.keys().next() {
                    console_log!("DO: Player {} left, declaring {} winner", player_id, remaining_player);
                    Self::broadcast_game_over(&gs, remaining_player);
                    gs.game_started = false;
                } else {
                    // Both left? Stop game.
                    gs.game_started = false;
                }
            }

            // Stop game if we lost a player
            if gs.clients.len() < 2 {
                gs.game_started = false;
            }
        }

        console_log!("DO: Remaining clients after cleanup: {}", gs.clients.len());
        Ok(())
    }

    async fn websocket_error(&self, _ws: WebSocket, error: Error) -> Result<()> {
        console_error!("DO: WebSocket error: {:?}", error);
        Ok(())
    }

    #[allow(clippy::await_holding_refcell_ref)] // We drop the RefCell borrow before await
    async fn alarm(&self) -> Result<Response> {
        // Game loop - runs at 60 Hz
        let tick_interval_ms = 16; // ~60 Hz

        let mut gs = self.game_state.borrow_mut();

        // Check for idle clients and disconnect them (1 minute timeout)
        let now = Date::now() as u64 / 1000; // Current time in seconds
        let idle_timeout_seconds = 60; // 1 minute
        let mut clients_to_remove = Vec::new();

        for (player_id, client_info) in gs.clients.iter() {
            if now.saturating_sub(client_info.last_activity) > idle_timeout_seconds {
                console_log!(
                    "DO: Client {} idle for {}s, disconnecting",
                    player_id,
                    now.saturating_sub(client_info.last_activity)
                );
                clients_to_remove.push(*player_id);
            }
        }

        // Remove idle clients
        for player_id in clients_to_remove {
            if let Some(_client_info) = gs.clients.remove(&player_id) {
                // Despawn paddle
                let entity_to_despawn =
                    gs.world
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
                    let _ = gs.world.despawn(entity);
                    console_log!("DO: Despawned paddle for idle player {}", player_id);
                }

                // Stop game if we lost a player
                if gs.clients.len() < 2 {
                    gs.game_started = false;
                }
            }
        }

        // Check if we still have clients after cleanup
        let has_clients = !gs.clients.is_empty();
        if !has_clients {
            console_log!("DO: No clients remaining, stopping alarm loop");
            drop(gs); // Release borrow before return
            return Response::ok("No clients, stopping alarm loop");
        }

        // Run simulation
        Self::step_game_simulation(&mut gs);

        // Release borrow before async call
        drop(gs);

        // Schedule next alarm
        // Note: We've dropped the game_state borrow, so this is safe
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
                    // Check if we already have 2 players
                    if gs.clients.len() >= 2 {
                        console_log!("DO: Match full, rejecting new player");
                        return Ok(());
                    }

                    // Assign player_id (0 = left, 1 = right)
                    let player_id = gs.next_player_id;
                    gs.next_player_id = (gs.next_player_id + 1) % 2; // Wrap at 2 players max

                    let was_empty = gs.clients.is_empty();
                    let now = Date::now() as u64 / 1000; // Current time in seconds
                    console_log!(
                        "DO: Player {} joining (clients was empty: {})",
                        player_id,
                        was_empty
                    );

                    gs.clients.insert(
                        player_id,
                        ClientInfo {
                            ws: ws.clone(),
                            last_activity: now,
                        },
                    );
                    console_log!("DO: Total clients: {}", gs.clients.len());

                    // Create paddle for this player
                    let paddle_y = gs.map.paddle_spawn(player_id).y;
                    create_paddle(&mut gs.world, player_id, paddle_y);

                    // Send Welcome message
                    let welcome = S2C::Welcome { player_id };
                    let bytes = welcome.to_bytes().map_err(|e| {
                        Error::RustError(format!("Failed to serialize Welcome: {e:?}"))
                    })?;
                    ws.send_with_bytes(&bytes)?;

                    // Start game when we have 2 players
                    if gs.clients.len() == 2 {
                        gs.game_started = true;
                        console_log!("DO: Game started with 2 players!");
                    }

                    // Send initial state
                    let state_msg = Self::generate_state_message(&gs);
                    let state_bytes = state_msg.to_bytes().map_err(|e| {
                        Error::RustError(format!("Failed to serialize GameState: {e:?}"))
                    })?;

                    // Broadcast to all clients so everyone knows someone joined
                    for client_info in gs.clients.values() {
                        let _ = client_info.ws.send_with_bytes(&state_bytes);
                    }

                    Some(was_empty)
                }
                C2S::Input {
                    player_id,
                    paddle_dir,
                    seq: _, // Client sequence number (not used by server)
                } => {
                    // Verify the player exists and update activity time
                    if let Some(client_info) = gs.clients.get_mut(&player_id) {
                        let now = js_sys::Date::now() as u64 / 1000;
                        client_info.last_activity = now;

                        // Only log when input changes (reduces log spam)
                        let last_dir = gs.last_input.get(&player_id).copied().unwrap_or(99);
                        if paddle_dir != last_dir {
                            console_log!(
                                "DO: Player {} input changed: {} -> {}",
                                player_id,
                                last_dir,
                                paddle_dir
                            );
                            gs.last_input.insert(player_id, paddle_dir);
                        }
                        gs.net_queue.push_input(player_id, paddle_dir);
                    } else {
                        console_log!(
                            "DO: Input from unknown player_id={}, paddle_dir={}",
                            player_id,
                            paddle_dir
                        );
                    }
                    None
                }
                C2S::Ping { t_ms } => {
                    // Update activity time for the client that sent the ping
                    // We need to find which client this WebSocket belongs to
                    // Since we can't compare WebSockets directly, we'll update all clients
                    // (this is safe - updating activity time multiple times is harmless)
                    let now = Date::now() as u64 / 1000;
                    for client_info in gs.clients.values_mut() {
                        client_info.last_activity = now;
                    }

                    // Send Pong response
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

    /// Step the game simulation
    fn step_game_simulation(gs: &mut GameState) {
        if !gs.game_started {
            return; // Wait for 2 players
        }

        gs.time.dt = 0.016; // ~60 Hz
        gs.tick += 1;

        if gs.tick % 60 == 0 {
            // Log every second
            console_log!(
                "DO: Game running, tick={}, clients={}",
                gs.tick,
                gs.clients.len()
            );
        }

        // Run Pong simulation
        game_core::step(
            &mut gs.world,
            &mut gs.time,
            &gs.map,
            &gs.config,
            &mut gs.score,
            &mut gs.events,
            &mut gs.net_queue,
            &mut gs.rng,
            &mut gs.respawn_state,
        );

        // Broadcast state at 20 Hz (every 3 ticks) instead of 60 Hz to reduce Durable Object requests
        // This reduces requests by 66% while still maintaining smooth gameplay
        // Always send on first tick (tick 1) to ensure clients get initial state
        if gs.tick == 1 || gs.tick % 3 == 0 {
            Self::broadcast_state(gs);
        }

        // Check win condition
        if let Some(winner) = gs.score.has_winner(gs.config.win_score) {
            Self::broadcast_game_over(gs, winner);
            gs.game_started = false; // Stop game
        }
    }

    /// Broadcast game state to all clients
    fn broadcast_state(gs: &mut GameState) {
        if gs.clients.is_empty() {
            return;
        }

        let state_msg = Self::generate_state_message(gs);
        let bytes = match state_msg.to_bytes() {
            Ok(b) => b,
            Err(_) => return,
        };

        let client_count = gs.clients.len();
        // Log broadcast frequency for monitoring (every 60 ticks = ~1 second at 20 Hz)
        if gs.tick % 60 == 0 {
            console_log!(
                "DO: Broadcasting state to {} clients (tick={}, ~{} broadcasts/sec)",
                client_count,
                gs.tick,
                if gs.tick > 0 { 20 } else { 0 }
            );
        }

        for client_info in gs.clients.values() {
            let _ = client_info.ws.send_with_bytes(&bytes);
        }
    }

    /// Generate GameState message from current game state
    fn generate_state_message(gs: &GameState) -> S2C {
        // Get ball position and velocity
        let (ball_x, ball_y, ball_vx, ball_vy) = gs
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

        for (_e, paddle) in gs.world.query::<&Paddle>().iter() {
            paddle_count += 1;
            if paddle.player_id == 0 {
                paddle_left_y = paddle.y;
            } else if paddle.player_id == 1 {
                paddle_right_y = paddle.y;
            }
        }

        if gs.tick % 60 == 0 {
            console_log!(
                "DO: Paddle state - count={}, left_y={:.1}, right_y={:.1}",
                paddle_count,
                paddle_left_y,
                paddle_right_y
            );
        }

        S2C::GameState {
            tick: gs.tick,
            ball_x,
            ball_y,
            ball_vx,
            ball_vy,
            paddle_left_y,
            paddle_right_y,
            score_left: gs.score.left,
            score_right: gs.score.right,
        }
    }

    /// Broadcast game over message
    fn broadcast_game_over(gs: &GameState, winner: u8) {
        let msg = S2C::GameOver { winner };
        if let Ok(bytes) = msg.to_bytes() {
            for client_info in gs.clients.values() {
                let _ = client_info.ws.send_with_bytes(&bytes);
            }
        }
    }
}
