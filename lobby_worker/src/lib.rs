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
        #canvas { border: 2px solid #fff; background: #000; display: block; }
        #score { position: absolute; top: 20px; left: 50%; transform: translateX(-50%); font-size: 48px; color: #fff; text-shadow: 0 0 10px #fff; pointer-events: none; }
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
    </div>
    <div id="ui">
        <div id="status">Initializing...</div>
        <div>
            <input type="text" id="matchCode" placeholder="MATCH CODE" maxlength="5">
            <button id="joinBtn" onclick="joinMatch()">JOIN</button>
            <button id="createBtn" onclick="createMatch()">CREATE</button>
        </div>
        <div class="controls">Controls: ↑/↓ or W/S to move your paddle</div>
    </div>
    <script type="module">
        import init, { WasmClient } from '/client_wasm/client_wasm.js';
        let client = null;
        let ws = null;
        let scoreLeft = 0;
        let scoreRight = 0;

        async function main() {
            try {
                await init();
                updateStatus('Ready to play!');
                document.getElementById('createBtn').disabled = false;
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

        function updateScore(left, right) {
            scoreLeft = left;
            scoreRight = right;
            const el = document.getElementById('score');
            if (el) el.textContent = `${left} : ${right}`;
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

        window.joinMatch = async function() {
            const code = document.getElementById('matchCode').value.trim().toUpperCase();
            if (code.length !== 5) { updateStatus('Match code must be 5 characters'); return; }
            try {
                updateStatus('Initializing client...');
                const canvas = document.getElementById('canvas');
                client = await new WasmClient(canvas);
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
                ws.onmessage = (event) => { if (event.data instanceof ArrayBuffer) { try { client.on_message(new Uint8Array(event.data)); } catch (e) { console.error('Message error:', e); } } };
                ws.onerror = (error) => { console.error('WS error:', error); updateStatus('Connection error'); };
                ws.onclose = () => { console.log('WS closed'); updateStatus('Disconnected'); };
            } catch (error) {
                console.error('Join error:', error);
                updateStatus('Error: ' + error.message);
            }
        };

        let renderLoopId = null;
        function startRender() {
            function render() {
                if (client) { try { client.render(); } catch (e) { console.error('Render error:', e); } }
                renderLoopId = requestAnimationFrame(render);
            }
            render();
        }

        function setupInput() {
            window.addEventListener('keydown', (e) => { if (client) client.on_key_down(e); });
            window.addEventListener('keyup', (e) => { if (client) client.on_key_up(e); });
            setInterval(() => {
                if (ws && ws.readyState === WebSocket.OPEN && client) {
                    try { ws.send(client.get_input_bytes()); } catch (e) { console.error('Input error:', e); }
                }
            }, 33);
        }

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
