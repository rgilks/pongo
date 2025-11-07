use game_core::*;
use hecs::World;
use proto::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::time::Duration;
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
    clients: HashMap<u16, WebSocket>, // player_id -> WebSocket
    #[allow(dead_code)]
    ws_to_player: HashMap<u32, u16>, // WebSocket ID -> player_id (temporary mapping - TODO: implement)
    next_player_id: u16,
    #[allow(dead_code)]
    next_ws_id: u32,
    #[allow(dead_code)]
    match_started: bool,
    #[allow(dead_code)]
    last_tick_ms: u64,
    snapshot_id: u32,
    tick: u32,
    // Snapshot throttle: only send snapshot every N ticks to reduce requests
    snapshot_throttle: u32,
    snapshot_throttle_counter: u32,
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
            ws_to_player: HashMap::new(),
            next_player_id: 1,
            next_ws_id: 1,
            match_started: false,
            last_tick_ms: 0,
            snapshot_id: 0,
            tick: 0,
            snapshot_throttle: 1, // Send snapshot every tick (1 = no throttle, 2 = every other tick, etc.)
            snapshot_throttle_counter: 0,
        };

        Self {
            state,
            env,
            game_state: RefCell::new(game_state),
        }
    }

    async fn fetch(&self, req: Request) -> Result<Response> {
        // Log request details for debugging
        console_log!("DO: Received request, method: {:?}", req.method());
        if let Ok(url) = req.url() {
            console_log!("DO: Request URL: {}", url);
        }

        // Check for Upgrade header
        let upgrade_header = req.headers().get("Upgrade");
        console_log!("DO: Upgrade header result: {:?}", upgrade_header);

        // Determine if this is a WebSocket upgrade request
        match upgrade_header {
            Ok(Some(header)) if header.to_lowercase() == "websocket" => {
                console_log!(
                    "DO: Received WebSocket upgrade request with header: {}",
                    header
                );

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
        // Handle incoming WebSocket messages
        match message {
            durable::WebSocketIncomingMessage::String(_text) => {
                // Text messages not used in protocol
                // Ignore or log
            }
            durable::WebSocketIncomingMessage::Binary(bytes) => {
                // Parse C2S message
                match C2S::from_bytes(&bytes) {
                    Ok(c2s_msg) => {
                        if let Err(e) = Self::handle_c2s_message(self, ws, c2s_msg).await {
                            // Log error but don't close connection
                            eprintln!("Error handling C2S message: {:?}", e);
                        }
                    }
                    Err(e) => {
                        // Log parse error but don't close connection
                        eprintln!("Failed to parse C2S message: {:?}", e);
                    }
                }
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
        // Note: In Cloudflare Workers, we can't directly compare WebSocket objects
        // We'll need to track this differently - for now, cleanup happens on error
        // TODO: Implement proper WebSocket tracking
        Ok(())
    }

    async fn websocket_error(&self, _ws: WebSocket, _error: Error) -> Result<()> {
        // Handle WebSocket errors
        // TODO: Log error and remove client
        Ok(())
    }

    async fn alarm(&self) -> Result<Response> {
        // Periodic game loop - runs every 200ms (5 ticks/sec) to reduce request volume
        // For production, use 50ms (20 ticks/sec)
        let tick_interval_ms = 200;

        // Check if we have clients and run simulation
        let has_clients = {
            let gs = self.game_state.borrow();
            !gs.clients.is_empty()
        };

        if has_clients {
            // Run simulation (borrow is dropped before this block)
            {
                let mut gs = self.game_state.borrow_mut();
                Self::step_game_simulation(&mut gs);
            } // Drop borrow before await

            // Schedule next alarm only if we have clients
            // This prevents unnecessary alarms when no one is connected
            self.state
                .storage()
                .set_alarm(Duration::from_millis(tick_interval_ms))
                .await?;

            Response::ok("Alarm processed")
        } else {
            // No clients - don't schedule another alarm to save requests
            // Alarm will restart when a player joins
            Response::ok("No clients, stopping alarm loop")
        }
    }
}

impl MatchDO {
    /// Handle incoming C2S message
    async fn handle_c2s_message(&self, ws: WebSocket, msg: C2S) -> Result<()> {
        let should_start_alarm = {
            let mut gs = self.game_state.borrow_mut();
            match msg {
                C2S::Join {
                    code: _,
                    avatar,
                    name_id,
                } => {
                    // Assign player_id
                    let player_id = gs.next_player_id;
                    gs.next_player_id = gs.next_player_id.wrapping_add(1);

                    // Store WebSocket connection
                    let was_empty = gs.clients.is_empty();
                    gs.clients.insert(player_id, ws.clone());

                    // Spawn player entity at a spawn point
                    let spawn_idx = (player_id as usize) % gs.map.map.spawns.len();
                    let spawn_pos = gs.map.map.spawns[spawn_idx];
                    create_player(&mut gs.world, player_id, avatar, name_id, spawn_pos);

                    // Send Welcome message
                    let welcome = S2C::Welcome {
                        player_id,
                        params_hash: 0, // TODO: Compute params hash
                        map_rev: 0,     // TODO: Map revision
                    };
                    let bytes = welcome.to_bytes().map_err(|e| {
                        Error::RustError(format!("Failed to serialize Welcome: {:?}", e))
                    })?;
                    ws.send_with_bytes(&bytes)?;

                    // Send initial snapshot so client can see game state
                    gs.snapshot_id += 1;
                    let snapshot = Self::generate_snapshot(&mut gs);
                    let snapshot_bytes = match snapshot.to_bytes() {
                        Ok(b) => b,
                        Err(e) => {
                            // Log error but don't fail - client can wait for next snapshot
                            eprintln!("Failed to serialize initial snapshot: {:?}", e);
                            return Ok(());
                        }
                    };
                    ws.send_with_bytes(&snapshot_bytes)?;

                    // Return whether we should start the alarm (drop borrow first)
                    Some(was_empty)
                }
                C2S::Input {
                    seq,
                    t_ms,
                    thrust_i8,
                    turn_i8,
                    bolt,
                    shield,
                } => {
                    // Find player_id for this WebSocket
                    // TODO: Proper WebSocket tracking - need to map WebSocket to player_id
                    // For now, we'll need to pass player_id in the message or track it differently
                    // This is a limitation of Cloudflare Workers WebSocket API
                    // Workaround: Store WebSocket when Join message arrives, then use that mapping
                    // For now, use the first player (this is a temporary workaround for testing)
                    let player_id = gs.clients.keys().next().copied();

                    if let Some(pid) = player_id {
                        // Convert i8 inputs to f32 (-127..127 -> -1.0..1.0)
                        let thrust = thrust_i8 as f32 / 127.0;
                        let turn = turn_i8 as f32 / 127.0;

                        // Add to net_queue (will be processed in next alarm tick)
                        gs.net_queue.inputs.push(InputEvent {
                            player_id: pid,
                            seq,
                            t_ms,
                            thrust,
                            turn,
                            bolt_level: bolt.min(3),
                            shield_level: shield.min(3),
                        });
                    }
                    None
                }
                C2S::Ping { t_ms: _ } => {
                    // TODO: Handle ping for latency measurement
                    None
                }
                C2S::Ack { snapshot_id: _ } => {
                    // TODO: Track ACKs for snapshot reliability
                    None
                }
            }
        };

        // Start game loop alarm if this was the first player (after borrow is dropped)
        // Use 200ms interval to reduce request volume during development
        if let Some(true) = should_start_alarm {
            self.state
                .storage()
                .set_alarm(Duration::from_millis(200))
                .await?;
        }

        Ok(())
    }

    /// Step the game simulation
    fn step_game_simulation(gs: &mut GameState) {
        // Update time (200ms step for reduced request volume)
        gs.time.dt = 0.2; // 200ms step
        gs.tick += 1;

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

        // Generate and broadcast snapshot (throttled to reduce requests)
        gs.snapshot_throttle_counter += 1;
        if gs.snapshot_throttle_counter >= gs.snapshot_throttle {
            gs.snapshot_throttle_counter = 0;
            gs.snapshot_id += 1;
            Self::broadcast_snapshot(gs);
        }

        // Clear processed inputs (they're consumed by ingest_inputs)
        gs.net_queue.inputs.clear();
        gs.net_queue.acks.clear();
    }

    /// Generate and broadcast snapshot to all connected clients
    fn broadcast_snapshot(gs: &mut GameState) {
        // Skip if no clients to reduce unnecessary work
        if gs.clients.is_empty() {
            return;
        }

        let snapshot = Self::generate_snapshot(gs);
        let bytes = match snapshot.to_bytes() {
            Ok(b) => b,
            Err(_) => return, // Skip if serialization fails
        };

        // Broadcast to all clients
        // Note: Each send might count as a request, but WebSocket sends are more efficient
        for ws in gs.clients.values() {
            // Note: In real implementation, we'd need to handle async send
            // For now, this is a placeholder
            let _ = ws.send_with_bytes(&bytes);
        }
    }

    /// Generate S2C Snapshot from current game state
    fn generate_snapshot(gs: &mut GameState) -> S2C {
        let mut players = Vec::new();
        let mut bolts = Vec::new();
        let mut pickups = Vec::new();

        // Collect player data
        for (_entity, (player, transform, velocity, health, energy, shield, bolt_max)) in gs
            .world
            .query::<(
                &Player,
                &Transform2D,
                &Velocity2D,
                &Health,
                &Energy,
                &Shield,
                &BoltMaxLevel,
            )>()
            .iter()
        {
            players.push(PlayerP {
                id: player.id,
                pos_q: [quantize_pos(transform.pos.x), quantize_pos(transform.pos.y)],
                vel_q: [quantize_pos(velocity.vel.x), quantize_pos(velocity.vel.y)],
                yaw_q: quantize_yaw(transform.yaw),
                bolt_max: bolt_max.level,
                shield_max: shield.max,
                hp: 3u8.saturating_sub(health.damage),
                energy_q: quantize_energy(energy.cur),
                flags: 0, // TODO: Add flags (e.g., respawn protection)
            });
        }

        // Collect bolt data
        for (entity, (bolt, transform, velocity)) in gs
            .world
            .query::<(&Bolt, &Transform2D, &Velocity2D)>()
            .iter()
        {
            bolts.push(BoltP {
                id: entity.id() as u16,
                pos_q: [quantize_pos(transform.pos.x), quantize_pos(transform.pos.y)],
                vel_q: [quantize_pos(velocity.vel.x), quantize_pos(velocity.vel.y)],
                rad_q: (bolt.radius * POS_SCALE) as u8,
                level: bolt.level,
                owner: bolt.owner,
            });
        }

        // Collect pickup data
        for (entity, (pickup, transform)) in gs.world.query::<(&Pickup, &Transform2D)>().iter() {
            pickups.push(PickupP {
                id: entity.id() as u16,
                pos_q: [quantize_pos(transform.pos.x), quantize_pos(transform.pos.y)],
                kind: match pickup.kind {
                    PickupKind::Health => 0,
                    PickupKind::BoltUpgrade => 1,
                    PickupKind::ShieldModule => 2,
                },
            });
        }

        // Find hill owner (simplified - check who's in hill zone)
        let hill_owner = Self::find_hill_owner(gs);

        S2C::Snapshot {
            id: gs.snapshot_id,
            tick: gs.tick,
            t_ms: (gs.time.now * 1000.0) as u32,
            last_seq_ack: 0, // TODO: Track last acked sequence per client
            players,
            bolts,
            pickups,
            hill_owner,
            hill_progress_u16: 0, // TODO: Calculate hill progress
        }
    }

    /// Find current hill owner (simplified)
    fn find_hill_owner(_gs: &GameState) -> Option<u16> {
        // TODO: Implement proper hill ownership check
        // For now, return None
        None
    }
}
