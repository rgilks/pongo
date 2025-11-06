# WebSocket Connection Status

## Current Problem

The WebSocket connection from the browser client to the Cloudflare Durable Object is failing with a **500 Internal Server Error** during the handshake phase.

**Error in browser console:**
```
WebSocket connection to 'wss://iso.rob-gilks.workers.dev/ws/:code' failed: 
Error during WebSocket handshake: Unexpected response code: 500
```

## Architecture

```
Browser (JavaScript)
  ↓ WebSocket upgrade request to /ws/:code
Cloudflare Worker (lobby_worker)
  ↓ Forward via stub.fetch_with_request(req)
Durable Object (MatchDO)
  ↓ Accept WebSocket, return 101 response
Browser (should receive 101 Switching Protocols)
```

## What We've Tried

### 1. Initial Implementation
- Forwarded WebSocket upgrade requests from Worker to DO
- Added error handling and logging
- Verified DO code structure matches Cloudflare patterns

### 2. Research Findings
Based on research, the common causes of 500 errors are:
- **Re-wrapping the 101 response** - Any modification of the response drops the WebSocket
- **Creating a new Request** instead of forwarding the original
- **Dev tooling issues** - Some dev servers don't support WS upgrade passthrough

### 3. Code Fixes Applied
- ✅ Removed all response modification/wrapping in Worker
- ✅ Return DO response directly: `stub.fetch_with_request(req).await`
- ✅ Simplified DO fetch handler to minimal pattern
- ✅ Use `Response::from_websocket(client)` directly without wrapping
- ✅ Removed status code checking that might modify response

## Current Code State

### Worker (`lobby_worker/src/lib.rs`)
```rust
async fn handle_websocket(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let code = ctx.param("code").map_or("", |v| v);
    if code.is_empty() || code.len() != 5 {
        return Response::error("Invalid match code", 400);
    }

    // Get DO stub and forward original request directly
    let match_do = ctx.env.durable_object("MATCH")?;
    let do_id = match_do.id_from_name(code)?;
    let stub = do_id.get_stub()?;

    // CRITICAL: Return DO response DIRECTLY without any modification
    stub.fetch_with_request(req).await
}
```

### Durable Object (`server_do/src/lib.rs`)
```rust
async fn fetch(&self, req: Request) -> Result<Response> {
    match req.headers().get("Upgrade") {
        Ok(Some(header)) if header.to_lowercase() == "websocket" => {
            let pair = WebSocketPair::new()?;
            let server = pair.server;
            let client = pair.client;

            // Accept WebSocket before returning response
            self.state.accept_web_socket(&server);

            // Return 101 response directly - do NOT wrap or modify
            Response::from_websocket(client)
        }
        _ => Response::error("Expected WebSocket upgrade request", 426)
    }
}
```

## Code Verification

The current code matches the **known-good Rust pattern** from research:
- ✅ Forwards original `Request` object (not a new one)
- ✅ Returns DO response directly without modification
- ✅ Uses `Response::from_websocket(client)` correctly
- ✅ Calls `accept_web_socket` before returning response
- ✅ No response wrapping, status checking, or header modification

## Possible Remaining Issues

1. **`accept_web_socket` may be panicking**
   - The method internally uses `unwrap()`, so failures cause panics
   - Panics would result in 500 errors
   - Need to verify if this is the actual failure point

2. **Compatibility date or configuration**
   - May need to verify `wrangler.toml` compatibility date
   - DO binding configuration might need adjustment

3. **Missing `websocket_open` handler**
   - Research example showed this handler
   - May be required for proper WebSocket initialization
   - Currently we only have `websocket_message`, `websocket_close`, `websocket_error`

4. **Runtime environment differences**
   - Testing on deployed environment (should be correct)
   - May need to test with `wrangler dev` for detailed error messages

## Next Steps

1. **Test locally with `wrangler dev`**
   - Should provide more detailed error messages
   - Can see if `accept_web_socket` is actually being called
   - Can verify if Upgrade header is reaching the DO

2. **Add `websocket_open` handler**
   - Implement the handler even if empty
   - May be required for proper WebSocket lifecycle

3. **Check Cloudflare logs**
   - Use `wrangler tail` during connection attempts
   - Look for panic messages or detailed error information

4. **Verify configuration**
   - Check `wrangler.toml` compatibility date
   - Verify DO binding is correct
   - Ensure no middleware is interfering

## Research References

- Cloudflare Workers Rust docs: `fetch_with_request` preserves headers
- Common pitfall: Modifying 101 response drops WebSocket
- Pattern: Forward original Request, return DO response unchanged
- `Response::from_websocket` constructs proper 101 response automatically

## Files Modified

- `lobby_worker/src/lib.rs` - Simplified WebSocket forwarding
- `server_do/src/lib.rs` - Simplified DO fetch handler
- Removed excessive logging and error handling that might wrap responses

## Test Status

- ✅ All unit tests passing
- ✅ All integration tests passing
- ✅ Code compiles without errors
- ✅ Clippy checks pass
- ❌ WebSocket connection still returns 500 error

---

**Last Updated:** 2025-11-06
**Status:** Code structure correct, but connection still failing. Need detailed error investigation.

