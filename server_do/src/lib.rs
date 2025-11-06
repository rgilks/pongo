use game_core::*;
use hecs::World;
use proto::*;
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

            // Note: WebSocket tracking is handled when Join message arrives
            // We can't store the WebSocket here because we need player_id from Join message

            // Client will send Join message after connection
            // We'll handle player assignment in websocket_message

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
            durable::WebSocketIncomingMessage::String(_text) => {
                // Text messages not used in protocol
                // Ignore or log
            }
            durable::WebSocketIncomingMessage::Binary(bytes) => {
                // Parse C2S message
                match C2S::from_bytes(&bytes) {
                    Ok(c2s_msg) => {
                        let mut gs = self.game_state.borrow_mut();
                        Self::handle_c2s_message(&mut gs, ws, c2s_msg)?;
                    }
                    Err(_e) => {
                        // Log parse error (for now, just ignore)
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
}

impl MatchDO {
    /// Handle incoming C2S message
    fn handle_c2s_message(gs: &mut GameState, ws: WebSocket, msg: C2S) -> Result<()> {
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

                    // Add to net_queue
                    gs.net_queue.inputs.push(InputEvent {
                        player_id: pid,
                        seq,
                        t_ms,
                        thrust,
                        turn,
                        bolt_level: bolt.min(3),
                        shield_level: shield.min(3),
                    });

                    // Trigger simulation step
                    Self::step_game_simulation(gs);
                }
            }
            C2S::Ping { t_ms: _ } => {
                // TODO: Handle ping for latency measurement
            }
            C2S::Ack { snapshot_id: _ } => {
                // TODO: Track ACKs for snapshot reliability
            }
        }
        Ok(())
    }

    /// Step the game simulation
    fn step_game_simulation(gs: &mut GameState) {
        // Update time
        gs.time.dt = 0.05; // 50ms step
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

        // Generate and broadcast snapshot
        gs.snapshot_id += 1;
        Self::broadcast_snapshot(gs);

        // Clear processed inputs (they're consumed by ingest_inputs)
        gs.net_queue.inputs.clear();
        gs.net_queue.acks.clear();
    }

    /// Generate and broadcast snapshot to all connected clients
    fn broadcast_snapshot(gs: &mut GameState) {
        let snapshot = Self::generate_snapshot(gs);
        let bytes = match snapshot.to_bytes() {
            Ok(b) => b,
            Err(_) => return, // Skip if serialization fails
        };

        // Broadcast to all clients
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
