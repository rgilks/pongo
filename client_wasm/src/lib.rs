//! WebGPU client for ISO game
//!
//! Engine-free rendering using wgpu for WebGPU API
//!
//! Based on geno-1 implementation using wgpu 24.0 with "webgpu" feature
//! Note: Canvas variant is only available when compiling for wasm32 target

#![cfg(target_arch = "wasm32")]

use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;
use wgpu::*;

/// Main client state
pub struct Client {
    device: Device,
    queue: Queue,
    surface: Surface<'static>,
    surface_config: SurfaceConfiguration,
    size: (u32, u32),
}

impl Client {
    /// Initialize WebGPU client
    pub async fn new(canvas: HtmlCanvasElement) -> Result<Self, JsValue> {
        // Get window size
        let width = canvas.width();
        let height = canvas.height();

        // Create instance with web backend
        let instance = Instance::default();

        // Create surface from canvas
        // Based on geno-1: wgpu::SurfaceTarget::Canvas(canvas.clone())
        // The "webgpu" feature + wasm32 target enables SurfaceTarget::Canvas variant
        let surface = instance
            .create_surface(wgpu::SurfaceTarget::Canvas(canvas.clone()))
            .map_err(|e| JsValue::from_str(&format!("Failed to create surface: {:?}", e)))?;

        // Request adapter
        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or_else(|| JsValue::from_str("Failed to get adapter"))?;

        // Request device
        let (device, queue) = adapter
            .request_device(
                &DeviceDescriptor {
                    label: None,
                    required_features: Features::empty(),
                    required_limits: Limits::default(),
                    memory_hints: MemoryHints::Performance,
                },
                None,
            )
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to get device: {}", e)))?;

        // Configure surface
        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| {
                matches!(
                    f,
                    TextureFormat::Bgra8UnormSrgb | TextureFormat::Rgba8UnormSrgb
                )
            })
            .unwrap_or(surface_caps.formats[0]);

        let surface_config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width,
            height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&device, &surface_config);

        Ok(Self {
            device,
            queue,
            surface,
            surface_config,
            size: (width, height),
        })
    }

    /// Resize the rendering surface
    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.size = (width, height);
            self.surface_config.width = width;
            self.surface_config.height = height;
            self.surface.configure(&self.device, &self.surface_config);
        }
    }

    /// Render a frame
    pub fn render(&mut self) -> Result<(), JsValue> {
        let output = self
            .surface
            .get_current_texture()
            .map_err(|e| JsValue::from_str(&format!("Failed to get current texture: {:?}", e)))?;

        let view = output
            .texture
            .create_view(&TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let _render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::BLACK),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

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
