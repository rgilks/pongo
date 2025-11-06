use worker::*;

#[durable_object]
pub struct MatchDO {
    state: State,
    #[allow(dead_code)]
    env: Env,
}

impl DurableObject for MatchDO {
    fn new(state: State, env: Env) -> Self {
        Self { state, env }
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

            // Send welcome message
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
                // TODO: Parse Input message from proto
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
        // TODO: Implement client tracking
        Ok(())
    }

    async fn websocket_error(&self, _ws: WebSocket, _error: Error) -> Result<()> {
        // Handle WebSocket errors
        // TODO: Log error and remove client
        Ok(())
    }
}
