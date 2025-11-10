use worker::*;

// Export the Durable Object from server_do
pub use server_do::MatchDO;

#[event(fetch)]
pub async fn main(req: Request, env: Env, _ctx: worker::Context) -> Result<Response> {
    let router = Router::new();

    router
        .get_async("/", handle_index)
        .get_async("/create", handle_create)
        .get_async("/join/:code", handle_join)
        .get_async("/ws/:code", handle_websocket)
        .run(req, env)
        .await
}

async fn handle_index(_req: Request, _ctx: RouteContext<()>) -> Result<Response> {
    // For now, return a simple HTML page
    // In production, this would be served from static assets or R2
    let html = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>ISO Game</title>
    <style>
        body { margin: 0; padding: 0; display: flex; flex-direction: column; align-items: center; justify-content: center; min-height: 100vh; background: #1a1a1a; color: #fff; font-family: monospace; }
        #canvas { border: 2px solid #444; background: #000; }
        #ui { margin-top: 20px; text-align: center; }
        #status { margin: 10px 0; padding: 10px; background: #333; border-radius: 4px; }
        input, button { padding: 8px 16px; margin: 5px; font-family: monospace; font-size: 14px; }
        button { background: #4a9eff; color: white; border: none; border-radius: 4px; cursor: pointer; }
        button:hover { background: #5aaeff; }
        button:disabled { background: #666; cursor: not-allowed; }
    </style>
</head>
<body>
    <canvas id="canvas" width="800" height="600"></canvas>
    <div id="ui">
        <div id="status">Loading WASM...</div>
        <div>
            <input type="text" id="matchCode" placeholder="Match code (5 chars)" maxlength="5" style="text-transform: uppercase;">
            <button id="joinBtn" onclick="joinMatch()">Join Match</button>
        </div>
        <div style="margin-top: 10px; font-size: 12px; color: #888;">
            Controls: W/S (move), A/D (turn), 1/2/3 (fire), Q/E/R (shield)
        </div>
    </div>
    <script type="module">
        import init, { init_client, connect_websocket, prepare_join, prepare_input, render_frame, handle_websocket_message } from '/client_wasm/client_wasm.js';

        let clientInitialized = false;
        let ws = null;
        let inputState = { thrust: 0, turn: 0, bolt: 0, shield: 0 };

        async function main() {
            try {
                await init();
                updateStatus('WASM loaded');
                const canvas = document.getElementById('canvas');
                if (!canvas) throw new Error('Canvas not found');
                await init_client(canvas);
                clientInitialized = true;
                updateStatus('Client initialized - Ready to join');
                setupInputHandlers();
                startRenderLoop();
            } catch (error) {
                console.error('Error:', error);
                updateStatus('Error: ' + error.message);
            }
        }

        function updateStatus(msg) {
            const el = document.getElementById('status');
            if (el) el.textContent = msg;
            console.log('Status:', msg);
        }

        function setupInputHandlers() {
            const keys = new Set();
            let lastSentInput = { thrust: 0, turn: 0, bolt: 0, shield: 0 };
            
            window.addEventListener('keydown', (e) => { keys.add(e.key.toLowerCase()); updateInputState(keys); });
            window.addEventListener('keyup', (e) => { keys.delete(e.key.toLowerCase()); updateInputState(keys); });
            
            // Send inputs at 30 Hz (33ms) instead of 60 Hz to reduce request costs
            // Only send when input state changes (coalescing) to further reduce costs
            setInterval(() => {
                if (ws && ws.readyState === WebSocket.OPEN && clientInitialized) {
                    // Check if input state has changed
                    const hasChanged = 
                        inputState.thrust !== lastSentInput.thrust ||
                        inputState.turn !== lastSentInput.turn ||
                        inputState.bolt !== lastSentInput.bolt ||
                        inputState.shield !== lastSentInput.shield;
                    
                    if (hasChanged) {
                        try { 
                            const inputBytes = prepare_input(inputState.thrust, inputState.turn, inputState.bolt, inputState.shield);
                            if (inputBytes && ws.readyState === WebSocket.OPEN) {
                                ws.send(inputBytes);
                                // Track last sent state
                                lastSentInput = { ...inputState };
                            }
                        } 
                        catch (e) { console.error('Send input error:', e); }
                    }
                }
            }, 33); // 30 Hz = ~33ms interval (was 16ms = ~60 Hz)
        }

        function updateInputState(keys) {
            inputState.thrust = keys.has('w') ? 1 : keys.has('s') ? -1 : 0;
            inputState.turn = keys.has('a') ? -1 : keys.has('d') ? 1 : 0;
            inputState.bolt = keys.has('1') ? 1 : keys.has('2') ? 2 : keys.has('3') ? 3 : 0;
            inputState.shield = keys.has('q') ? 1 : keys.has('e') ? 2 : keys.has('r') ? 3 : 0;
        }

        function startRenderLoop() {
            function render() {
                if (clientInitialized) {
                    try { render_frame(); } catch (e) { console.error('Render error:', e); }
                }
                requestAnimationFrame(render);
            }
            render();
        }

        window.joinMatch = async function() {
            const code = document.getElementById('matchCode').value.trim().toUpperCase();
            if (code.length !== 5) { updateStatus('Match code must be 5 characters'); return; }
            if (!clientInitialized) { updateStatus('Client not initialized'); return; }
            try {
                document.getElementById('joinBtn').disabled = true;
                updateStatus('Connecting to match...');
                
                // Construct WebSocket URL
                const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
                const wsUrl = `${protocol}//${window.location.host}/ws/${code}`;
                
                // Create WebSocket connection
                ws = new WebSocket(wsUrl);
                
                ws.onopen = () => {
                    updateStatus('Connected! Sending join message...');
                    // Send join message via WASM
                    try {
                        connect_websocket(wsUrl); // Register with WASM client
                        const joinBytes = prepare_join(code, 0, 0); // avatar=0, name_id=0
                        if (joinBytes && ws.readyState === WebSocket.OPEN) {
                            ws.send(joinBytes);
                            updateStatus('Joined match! Waiting for game state...');
                        }
                    } catch (e) {
                        console.error('Join error:', e);
                        updateStatus('Error joining: ' + e.message);
                    }
                };
                
                ws.onmessage = async (event) => {
                    let bytes;
                    if (event.data instanceof ArrayBuffer) {
                        bytes = new Uint8Array(event.data);
                    } else if (event.data instanceof Blob) {
                        // Convert Blob to ArrayBuffer
                        const arrayBuffer = await event.data.arrayBuffer();
                        bytes = new Uint8Array(arrayBuffer);
                    } else {
                        console.warn('Client: Received unsupported message type:', event.data);
                        return;
                    }
                    console.log('Client: Received WebSocket message,', bytes.length, 'bytes');
                    try {
                        handle_websocket_message(bytes);
                        // Update status if we received a snapshot (indicates game state received)
                        if (bytes.length > 20) { // Snapshots are larger than Welcome messages
                            updateStatus('Game state received!');
                        }
                    } catch (e) {
                        console.error('Message handling error:', e);
                    }
                };
                
                ws.onerror = (error) => {
                    console.error('WebSocket error:', error);
                    updateStatus('WebSocket connection error');
                    document.getElementById('joinBtn').disabled = false;
                    // Close connection to ensure cleanup
                    if (ws) {
                        ws.close();
                        ws = null;
                    }
                };
                
                ws.onclose = () => {
                    updateStatus('Connection closed');
                    document.getElementById('joinBtn').disabled = false;
                    ws = null; // Clear reference to prevent reconnection loops
                };
                
            } catch (error) {
                updateStatus('Error: ' + error.message);
                document.getElementById('joinBtn').disabled = false;
            }
        };

        main();
    </script>
</body>
</html>"#;
    Response::from_html(html)
}

async fn handle_create(_req: Request, ctx: RouteContext<()>) -> Result<Response> {
    // Generate a random 5-character match code
    let code = generate_match_code();

    // Get the MATCH Durable Object namespace
    let match_do = ctx.env.durable_object("MATCH")?;

    // Get DO stub by name (creates if doesn't exist)
    let _stub = match_do.get_by_name(&code)?;

    // Return the match code to the client
    Response::ok(format!("Match created: {}", code))
}

async fn handle_join(_req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let code = ctx.param("code").map_or("", |v| v);

    if code.is_empty() || code.len() != 5 {
        return Response::error("Invalid match code", 400);
    }

    // Get the MATCH Durable Object namespace
    let match_do = ctx.env.durable_object("MATCH")?;

    // Get DO stub by name (this creates the DO if it doesn't exist)
    let _stub = match_do.get_by_name(code)?;

    // Return response with WebSocket URL
    Response::ok(format!(
        "Match {} found. Connect via WebSocket at /ws/{}",
        code, code
    ))
}

async fn handle_websocket(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let code = ctx.param("code").map_or("", |v| v);

    if code.is_empty() || code.len() != 5 {
        return Response::error("Invalid match code", 400);
    }

    // Get the Durable Object stub
    let match_do = ctx.env.durable_object("MATCH")?;
    let do_id = match_do.id_from_name(code)?;

    console_log!("Worker: Getting stub for DO with code {}", code);
    let stub = match do_id.get_stub() {
        Ok(s) => s,
        Err(e) => {
            console_error!("Worker: Failed to get stub: {:?}", e);
            return Response::error(format!("Failed to get DO stub: {:?}", e), 500);
        }
    };

    console_log!(
        "Worker: Forwarding WebSocket upgrade request to DO for code {}",
        code
    );

    // Log request details for debugging
    console_log!("Worker: Request method: {:?}", req.method());
    if let Ok(url) = req.url() {
        console_log!("Worker: Request URL: {}", url);
    }
    if let Ok(upgrade) = req.headers().get("Upgrade") {
        console_log!("Worker: Upgrade header: {:?}", upgrade);
    } else {
        console_log!("Worker: No Upgrade header found");
    }
    if let Ok(connection) = req.headers().get("Connection") {
        console_log!("Worker: Connection header: {:?}", connection);
    }

    // Ensure request method is GET (required for WebSocket upgrade)
    if req.method() != Method::Get {
        console_error!(
            "Worker: WebSocket upgrade requires GET method, got: {:?}",
            req.method()
        );
        return Response::error("WebSocket upgrade requires GET method", 405);
    }

    // Forward the original Request to the DO using fetch_with_request
    // This should preserve all headers including Upgrade and Connection for WebSocket upgrade
    console_log!("Worker: About to call fetch_with_request");
    match stub.fetch_with_request(req).await {
        Ok(resp) => {
            console_log!(
                "Worker: DO responded to WebSocket upgrade for code {}",
                code
            );
            Ok(resp)
        }
        Err(err) => {
            let err_str = format!("{:?}", err);
            console_error!(
                "Worker: Error forwarding WebSocket upgrade for code {}: {}",
                code,
                err_str
            );

            // Check if this is a rate limit error (free tier limitation)
            if err_str.contains("Exceeded allowed volume") || err_str.contains("free tier") {
                Response::error(
                    "Service temporarily unavailable due to rate limits. Please try again later.",
                    503,
                )
            } else {
                Response::error(
                    format!("Worker failed to forward WebSocket request: {}", err_str),
                    500,
                )
            }
        }
    }
}

/// Generate a random 5-character match code (A-Z, 0-9)
fn generate_match_code() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    (0..5)
        .map(|_| {
            let idx = rng.gen_range(0..CHARS.len());
            CHARS[idx] as char
        })
        .collect()
}
