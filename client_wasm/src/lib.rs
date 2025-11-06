//! WebGPU client for ISO game
//!
//! Engine-free rendering using wgpu for WebGPU API
//!
//! Note: WebGPU surface creation API for web/WASM is being researched.
//! See WEBGPU_RESEARCH.md for detailed findings and blockers.

use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;

/// Main client state
///
/// TODO: Complete WebGPU initialization once correct wgpu web API is verified
pub struct Client {
    _canvas: HtmlCanvasElement,
    _width: u32,
    _height: u32,
}

impl Client {
    /// Initialize WebGPU client
    ///
    /// TODO: Implement proper wgpu surface creation for web
    /// Current blocker: HtmlCanvasElement -> SurfaceTarget conversion API
    /// See WEBGPU_RESEARCH.md for detailed research findings
    pub async fn new(canvas: HtmlCanvasElement) -> Result<Self, JsValue> {
        let width = canvas.width();
        let height = canvas.height();

        // Placeholder implementation
        // Will be completed once we verify the correct wgpu web API
        // See WEBGPU_RESEARCH.md for attempted approaches and blockers
        Ok(Self {
            _canvas: canvas,
            _width: width,
            _height: height,
        })
    }

    /// Resize the rendering surface
    pub fn resize(&mut self, _width: u32, _height: u32) {
        // TODO: Implement resize once surface is created
    }

    /// Render a frame
    pub fn render(&mut self) -> Result<(), JsValue> {
        // TODO: Implement rendering once WebGPU is initialized
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
