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
    let html = include_str!("../index.html");
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
