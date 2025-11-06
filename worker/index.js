import init, {
  fetch,
  MatchDO,
} from "../lobby_worker/worker/pkg/lobby_worker.js";
import wasmUrl from "../lobby_worker/worker/pkg/lobby_worker_bg.wasm";

// Initialize WASM module - pass WASM URL explicitly for Workers
let initialized = false;

async function ensureInit() {
  if (!initialized) {
    // Pass the WASM URL explicitly to avoid import.meta.url issues
    await init(wasmUrl);
    initialized = true;
  }
}

// Export fetch handler
export default {
  async fetch(req, env, ctx) {
    try {
      await ensureInit();
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
export { MatchDO };
