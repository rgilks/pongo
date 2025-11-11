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
    let html = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Pong</title>
    <style>
        body { margin: 0; padding: 0; display: flex; flex-direction: column; align-items: center; justify-content: center; min-height: 100vh; background: #000; color: #fff; font-family: 'Courier New', monospace; }
        #gameContainer { position: relative; }
        #canvas { border: 2px solid #fff; display: block; }
        #score { position: absolute; top: 20px; left: 50%; transform: translateX(-50%); font-size: 48px; color: #fff; text-shadow: 0 0 10px #fff; pointer-events: none; }
        #metrics { position: absolute; top: 20px; right: 20px; font-size: 13px; color: #fff; text-shadow: 0 0 5px #fff; pointer-events: none; background: rgba(0, 0, 0, 0.7); padding: 8px 12px; border-radius: 4px; font-family: 'Courier New', monospace; border: 1px solid rgba(255, 255, 255, 0.2); }
        .metric-row { display: flex; align-items: center; margin: 4px 0; white-space: nowrap; }
        .metric-label { color: #aaa; margin-right: 8px; min-width: 50px; }
        .metric-value { font-weight: bold; color: #0f0; min-width: 50px; text-align: right; margin-right: 4px; }
        .metric-unit { color: #888; font-size: 11px; }
        #ui { margin-top: 30px; text-align: center; }
        #status { margin: 15px 0; padding: 10px 20px; background: #222; border: 2px solid #fff; border-radius: 4px; font-size: 16px; }
        input, button { padding: 10px 20px; margin: 5px; font-family: 'Courier New', monospace; font-size: 16px; border: 2px solid #fff; background: #000; color: #fff; }
        button { cursor: pointer; transition: all 0.2s; }
        button:hover:not(:disabled) { background: #fff; color: #000; }
        button:disabled { opacity: 0.5; cursor: not-allowed; }
        .controls { margin-top: 20px; font-size: 14px; color: #888; }
        #matchCode { text-transform: uppercase; }
    </style>
</head>
<body>
    <div id="gameContainer">
        <canvas id="canvas" width="800" height="600"></canvas>
        <div id="score">0 : 0</div>
        <div id="metrics">
            <div class="metric-row"><span class="metric-label">FPS:</span><span class="metric-value" id="fps">--</span></div>
            <div class="metric-row"><span class="metric-label">Ping:</span><span class="metric-value" id="ping">--</span><span class="metric-unit">ms</span></div>
            <div class="metric-row"><span class="metric-label">Update:</span><span class="metric-value" id="update">--</span><span class="metric-unit">ms</span></div>
        </div>
    </div>
    <div id="ui">
        <div id="status">Initializing...</div>
        <div>
            <input type="text" id="matchCode" placeholder="MATCH CODE" maxlength="5">
            <button id="joinBtn">JOIN</button>
            <button id="createBtn">CREATE</button>
            <button id="localBtn">VS AI</button>
        </div>
        <div class="controls">Controls: ↑/↓ or W/S to move your paddle</div>
    </div>
    <script type="module">
        // Cache busting for local development: use timestamp to force reload
        // This ensures browser always loads latest WASM files during development
        const CACHE_BUST = new URLSearchParams(window.location.search).get('v') || Date.now();
        const wasmUrl = '/client_wasm/client_wasm.js?v=' + CACHE_BUST;
        
        let init, WasmClient;
        try {
            console.log('📦 Loading WASM module from:', wasmUrl);
            const module = await import(wasmUrl);
            init = module.default;
            WasmClient = module.WasmClient;
            console.log('✅ WASM module loaded successfully');
        } catch (error) {
            console.error('❌ Failed to load WASM module:', error);
            const statusEl = document.getElementById('status');
            if (statusEl) {
                statusEl.textContent = 'Error: Failed to load game (check console)';
            }
            throw error;
        }
        
        let client = null;
        let ws = null;
        let scoreLeft = 0;
        let scoreRight = 0;

        function updateStatus(msg) {
            const el = document.getElementById('status');
            if (el) el.textContent = msg;
            console.log('Status:', msg);
        }

        function updateScore(left, right) {
            scoreLeft = left;
            scoreRight = right;
            const el = document.getElementById('score');
            if (el) el.textContent = `${left} : ${right}`;
        }

        function updateMetrics() {
            if (client) {
                try {
                    const metrics = client.get_metrics();
                    if (metrics.length >= 3) {
                        const fpsEl = document.getElementById('fps');
                        const pingEl = document.getElementById('ping');
                        const updateEl = document.getElementById('update');
                        if (fpsEl) fpsEl.textContent = Math.round(metrics[0]);
                        if (pingEl) pingEl.textContent = Math.round(metrics[1]);
                        if (updateEl) updateEl.textContent = Math.round(metrics[2]);
                    }
                } catch (e) {
                    console.error('Metrics error:', e);
                }
            }
        }

        window.createMatch = async function() {
            try {
                updateStatus('Creating match...');
                const response = await fetch('/create');
                const data = await response.json();
                document.getElementById('matchCode').value = data.code;
                updateStatus(`Match created: ${data.code}`);
                await joinMatch();
            } catch (error) {
                console.error('Create error:', error);
                updateStatus('Error creating match');
            }
        };

        window.startLocalGame = async function() {
            try {
                updateStatus('Starting local game...');
                const canvas = document.getElementById('canvas');
                // Ensure canvas has correct size
                if (!canvas.width || !canvas.height) {
                    canvas.width = 800;
                    canvas.height = 600;
                }
                if (!client) {
                    client = await new WasmClient(canvas);
                    console.log('✅ Client initialized');
                }
                client.start_local_game();
                updateStatus('Playing vs AI');
                setupInput();
                startRender();
            } catch (error) {
                console.error('Local game error:', error);
                updateStatus('Error starting local game');
            }
        };

        window.joinMatch = async function() {
            const code = document.getElementById('matchCode').value.trim().toUpperCase();
            if (code.length !== 5) { updateStatus('Match code must be 5 characters'); return; }
            try {
            updateStatus('Initializing client...');
            const canvas = document.getElementById('canvas');
            // Ensure canvas has correct size
            if (!canvas.width || !canvas.height) {
                canvas.width = 800;
                canvas.height = 600;
            }
            console.log('🔍 Canvas setup:', {
                width: canvas.width,
                height: canvas.height,
                clientWidth: canvas.clientWidth,
                clientHeight: canvas.clientHeight
            });
            // Don't request WebGPU context here - wgpu will handle it
            // Requesting it here can conflict with wgpu's surface creation
            client = await new WasmClient(canvas);
            console.log('✅ Client initialized');
            updateStatus('Connecting...');
            const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
            const wsUrl = `${protocol}//${window.location.host}/ws/${code}`;
            ws = new WebSocket(wsUrl);
            ws.binaryType = 'arraybuffer';
            ws.onopen = () => { 
                console.log('WS connected'); 
                try { 
                    ws.send(client.get_join_bytes(code)); 
                    console.log('Join sent'); 
                    updateStatus('Connected! Waiting for opponent...'); 
                } catch(e) { 
                    console.error('Join error:', e); 
                    updateStatus('Error joining'); 
                    return; 
                } 
                setupInput(); 
                startRender(); 
            };
            ws.onmessage = (event) => { 
                if (event.data instanceof ArrayBuffer) { 
                    try { 
                        client.on_message(new Uint8Array(event.data)); 
                        // Update score display
                        const score = client.get_score();
                        if (score.length >= 2) {
                            updateScore(score[0], score[1]);
                        }
                    } catch (e) { 
                        console.error('Message error:', e); 
                    } 
                } 
            };
            ws.onerror = (error) => { 
                console.error('WS error:', error); 
                updateStatus('Connection error'); 
            };
            ws.onclose = () => { 
                console.log('WS closed'); 
                updateStatus('Disconnected'); 
            };
            } catch (error) {
                console.error('Join error:', error);
                updateStatus('Error: ' + error.message);
            }
        };

        let renderLoopId = null;
        let pingIntervalId = null;
        function startRender() {
            console.log('startRender called, client exists:', !!client);
            function render() {
                if (client) { 
                    try { 
                        client.render();
                        updateMetrics(); // Update metrics display every frame
                        // Update score display (works for both online and local games)
                        const score = client.get_score();
                        if (score.length >= 2) {
                            updateScore(score[0], score[1]);
                        }
                    } catch (e) { 
                        console.error('Render error:', e); 
                    } 
                } else {
                    console.warn('Render called but client is null');
                }
                renderLoopId = requestAnimationFrame(render);
            }
            render();
            
            // Send ping every 2 seconds to measure latency
            pingIntervalId = setInterval(() => {
                if (ws && ws.readyState === WebSocket.OPEN && client) {
                    try {
                        const pingBytes = client.send_ping();
                        ws.send(pingBytes);
                    } catch (e) {
                        console.error('Ping error:', e);
                    }
                }
            }, 2000);
        }

        function sendInput() {
            if (ws && ws.readyState === WebSocket.OPEN && client) {
                try { 
                    const bytes = client.get_input_bytes();
                    if (bytes.length > 0) {
                        ws.send(bytes); 
                    }
                } catch (e) { 
                    console.error('Input error:', e); 
                }
            }
        }

        function setupInput() {
            window.addEventListener('keydown', (e) => { 
                if (client) { 
                    client.on_key_down(e); 
                    sendInput(); // Send immediately on keydown
                } 
            });
            window.addEventListener('keyup', (e) => { 
                if (client) { 
                    client.on_key_up(e); 
                    sendInput(); // Send immediately on keyup
                } 
            });
            // Also send periodically (for holding keys)
            setInterval(sendInput, 33);
        }

        async function main() {
            try {
                console.log('🚀 Starting main()...');
                await init();
                console.log('✅ WASM initialized');
                updateStatus('Ready to play!');
                const createBtn = document.getElementById('createBtn');
                const joinBtn = document.getElementById('joinBtn');
                const localBtn = document.getElementById('localBtn');
                createBtn.disabled = false;
                joinBtn.disabled = false;
                if (localBtn) localBtn.disabled = false;
                // Use event listeners instead of onclick to avoid timing issues
                createBtn.addEventListener('click', window.createMatch);
                joinBtn.addEventListener('click', window.joinMatch);
                if (localBtn) localBtn.addEventListener('click', window.startLocalGame);
                console.log('✅ UI initialized');
            } catch (error) {
                console.error('❌ Error in main():', error);
                updateStatus('Error: ' + error.message);
            }
        }

        // Start immediately (top-level await in module)
        main().catch(error => {
            console.error('❌ Fatal error:', error);
            document.getElementById('status').textContent = 'Fatal error: ' + error.message;
        });
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

    // Return JSON response with match code
    Response::from_json(&serde_json::json!({
        "code": code
    }))
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
