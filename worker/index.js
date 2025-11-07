import init, {
  fetch,
  MatchDO as WasmMatchDO,
} from "../lobby_worker/worker/pkg/lobby_worker.js";
import wasmUrl from "../lobby_worker/worker/pkg/lobby_worker_bg.wasm";

// Initialize WASM module - pass WASM URL explicitly for Workers
let initPromise;

async function ensureInit() {
  if (!initPromise) {
    initPromise = init(wasmUrl);
  }
  await initPromise;
}

// Export fetch handler
export default {
  async fetch(req, env, ctx) {
    try {
      await ensureInit();
      
      // First, try to serve static assets from ASSETS binding
      // This handles WASM files, JS files, etc.
      if (env.ASSETS) {
        const url = new URL(req.url);
        // Check if this is a static asset request (not a Worker route)
        if (url.pathname.startsWith("/client_wasm/") || 
            url.pathname.startsWith("/lobby_worker") ||
            url.pathname.endsWith(".wasm") ||
            url.pathname.endsWith(".js") ||
            url.pathname.endsWith(".d.ts")) {
          const assetResponse = await env.ASSETS.fetch(req);
          if (assetResponse.ok) {
            // Add cache headers
            const res = new Response(assetResponse.body, assetResponse);
            if (url.pathname.endsWith(".wasm")) {
              res.headers.set("Cache-Control", "public, max-age=86400");
            } else if (url.pathname.endsWith(".js") || url.pathname.endsWith(".d.ts")) {
              res.headers.set("Cache-Control", "public, max-age=3600");
            }
            return res;
          }
        }
      }
      
      // Otherwise, handle via Rust Worker
      return await fetch(req, env, ctx);
    } catch (error) {
      return new Response(`Error: ${error.message}\n${error.stack}`, {
        status: 500,
        headers: { "Content-Type": "text/plain" },
      });
    }
  },
};

// Export the Durable Object
class MatchDOWrapper {
  constructor(state, env) {
    this._inner = ensureInit().then(() => new WasmMatchDO(state, env));
  }

  async fetch(req) {
    const inner = await this._inner;
    return inner.fetch(req);
  }

  async alarm() {
    const inner = await this._inner;
    return inner.alarm();
  }

  async webSocketMessage(ws, message) {
    const inner = await this._inner;
    return inner.webSocketMessage(ws, message);
  }

  async webSocketClose(ws, code, reason, wasClean) {
    const inner = await this._inner;
    return inner.webSocketClose(ws, code, reason, wasClean);
  }

  async webSocketError(ws, error) {
    const inner = await this._inner;
    return inner.webSocketError(ws, error);
  }
}

export { MatchDOWrapper as MatchDO };
