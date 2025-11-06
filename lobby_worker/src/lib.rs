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
        import init, { init_client, connect_websocket, send_join, send_input, render_frame, handle_websocket_message } from './pkg/client_wasm.js';

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
            window.addEventListener('keydown', (e) => { keys.add(e.key.toLowerCase()); updateInputState(keys); });
            window.addEventListener('keyup', (e) => { keys.delete(e.key.toLowerCase()); updateInputState(keys); });
            setInterval(() => {
                if (ws && ws.readyState === WebSocket.OPEN && clientInitialized) {
                    try { send_input(inputState.thrust, inputState.turn, inputState.bolt, inputState.shield); } 
                    catch (e) { console.error('Send input error:', e); }
                }
            }, 16);
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
                updateStatus('Connecting...');
                // Note: WebSocket URL needs to be constructed properly for Cloudflare Workers
                // For now, this is a placeholder - actual connection logic needed
                updateStatus('WebSocket connection not yet implemented for browser client');
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

    // For now, return a simple response indicating the match exists
    // WebSocket connections should be made directly to the DO
    Response::ok(format!(
        "Match {} found. Connect via WebSocket to join.",
        code
    ))
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
