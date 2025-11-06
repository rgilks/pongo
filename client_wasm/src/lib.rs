//! WebGPU client for ISO game
//!
//! Engine-free rendering using wgpu for WebGPU API
//!
//! Note: WebGPU surface creation API is being finalized.
//! This is a placeholder structure that will be completed in the next iteration.

use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;

/// Main client state
///
/// TODO: Complete WebGPU initialization once wgpu web API is confirmed
pub struct Client {
    // Placeholder - will contain Device, Queue, Surface, etc.
    _canvas: HtmlCanvasElement,
}

impl Client {
    /// Initialize WebGPU client
    ///
    /// TODO: Implement proper wgpu surface creation for web
    pub async fn new(canvas: HtmlCanvasElement) -> Result<Self, JsValue> {
        // Placeholder implementation
        // Will be completed once we verify the correct wgpu web API
        Ok(Self { _canvas: canvas })
    }

    /// Resize the rendering surface
    pub fn resize(&mut self, _width: u32, _height: u32) {
        // TODO: Implement resize
    }

    /// Render a frame
    pub fn render(&mut self) -> Result<(), JsValue> {
        // TODO: Implement rendering
        Ok(())
    }
}

#[wasm_bindgen]
pub fn init_client(canvas: HtmlCanvasElement) -> js_sys::Promise {
    wasm_bindgen_futures::future_to_promise(async move {
        match Client::new(canvas).await {
            Ok(_client) => {
                // Store client in a way that can be accessed later
                // For now, just return success
                Ok(JsValue::UNDEFINED)
            }
            Err(e) => Err(e),
        }
    })
}
