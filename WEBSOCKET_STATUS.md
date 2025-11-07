# WebSocket Connection Status

## Current Status

- ✅ Browser → Worker → Durable Object WebSocket handshake now succeeds (returns 101 Switching Protocols).
- ✅ Client receives `Connected!` / `Joined match! Waiting for game state...` status updates.
- 🔄 Next milestone: feed game snapshots to the client so that the renderer updates (currently waiting for game state indefinitely).

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

## What Changed

### 1. Diagnose the 500 error
- Captured Worker error message: `TypeError: Cannot read properties of undefined (reading 'matchdo_new')`
- Determined that the Durable Object was being constructed before the WASM module finished initialising.

### 2. Fixes implemented
- Added a wrapper in `worker/index.js` that ensures `init(wasmUrl)` completes before the Durable Object class is instantiated (`MatchDOWrapper`).
- Returned more descriptive errors from the Worker during debugging (no longer required in production, but useful for future investigations).
- Corrected the Durable Object migration section in `wrangler.toml` to use `new_classes`.

## Key Files (post-fix)

### Worker (`worker/index.js`)
```javascript
import init, { fetch, MatchDO as WasmMatchDO } from "../lobby_worker/worker/pkg/lobby_worker.js";

let initPromise;
async function ensureInit() {
  if (!initPromise) initPromise = init(wasmUrl);
  await initPromise;
}

class MatchDOWrapper {
  constructor(state, env) {
    this._inner = ensureInit().then(() => new WasmMatchDO(state, env));
  }

  async fetch(req) {
    const inner = await this._inner;
    return inner.fetch(req);
  }

  async webSocketMessage(ws, message) {
    const inner = await this._inner;
    return inner.webSocketMessage(ws, message);
  }

  // ...alarm, webSocketClose, webSocketError similar...
}

export { MatchDOWrapper as MatchDO };
```

### Worker route (`lobby_worker/src/lib.rs`)
```rust
let match_do = ctx.env.durable_object("MATCH")?;
let do_id = match_do.id_from_name(code)?;
let stub = do_id.get_stub()?;

match stub.fetch_with_request(req).await {
    Ok(resp) => {
        console_log!("Worker: DO responded to WebSocket upgrade for code {}", code);
        Ok(resp)
    }
    Err(err) => {
        Response::error(
            format!("Worker failed to forward WebSocket request: {:?}", err),
            500,
        )
    }
}
```

### Durable Object (`server_do/src/lib.rs`)
```rust
match req.headers().get("Upgrade") {
    Ok(Some(header)) if header.to_lowercase() == "websocket" => {
        console_log!("DO: Received WebSocket upgrade request");
        let pair = WebSocketPair::new()?;
        self.state.accept_web_socket(&pair.server);
        Response::from_websocket(pair.client)
    }
    Ok(other) => {
        console_error!("DO: Unexpected Upgrade header state: {:?}", other);
        Response::error("Expected WebSocket upgrade request", 426)
    }
    Err(err) => {
        console_error!("DO: Failed to read Upgrade header: {:?}", err);
        Response::error("Failed to read request headers", 500)
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

## Observations & Remaining Work

- Worker and DO now exchange messages without runtime errors.
- Client UI stays on "Waiting for game state..." because the server currently sends a welcome + snapshot but the renderer pipeline still expects more data plumbing (future task).
- Logging instrumentation (`console_log!` / `console_error!`) stays in place until the snapshot/render loop is verified.

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

- ✅ `npm run test:all` (fmt + clippy + cargo test)
- ✅ Manual browser verification (`/create` + `/ws/:code`) shows successful WebSocket handshake
- ✅ Deployed version ID: `bd9117fb-228f-4703-a6bf-28c871fc9817`
- 🔄 Pending: integrate game snapshot flow + renderer update

---

**Last Updated:** 2025-11-07
**Status:** WebSocket handshake fixed (match join succeeds). Continue with game-state synchronisation next.

