//! WebGPU client for Pong game
//!
//! Simple 2D rendering using wgpu 24.0

#![cfg(target_arch = "wasm32")]

mod camera;
mod mesh;

use camera::{Camera, CameraUniform};
use mesh::{create_circle, create_rectangle, Mesh};
use proto::{C2S, S2C};
use wasm_bindgen::prelude::*;
use web_sys::{HtmlCanvasElement, KeyboardEvent};
use wgpu::util::DeviceExt;
use wgpu::*;

/// Instance data for rendering (matches shader InstanceData)
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct InstanceData {
    transform: [f32; 4], // x, y, scale_x, scale_y
    tint: [f32; 4],      // rgba
}

/// Game state tracking
struct GameState {
    ball_x: f32,
    ball_y: f32,
    paddle_left_y: f32,
    paddle_right_y: f32,
    score_left: u8,
    score_right: u8,
    my_player_id: Option<u8>,
}

impl GameState {
    fn new() -> Self {
        Self {
            ball_x: 16.0,
            ball_y: 12.0,
            paddle_left_y: 12.0,
            paddle_right_y: 12.0,
            score_left: 0,
            score_right: 0,
            my_player_id: None,
        }
    }
}

/// Main client state
pub struct Client {
    device: Device,
    queue: Queue,
    surface: Surface<'static>,
    surface_config: SurfaceConfiguration,
    size: (u32, u32),
    camera: Camera,
    camera_buffer: Buffer,
    camera_bind_group: BindGroup,
    camera_bind_group_layout: BindGroupLayout,
    // Meshes
    rectangle_mesh: Mesh,
    circle_mesh: Mesh,
    // Render pipeline
    render_pipeline: RenderPipeline,
    // Instance buffers
    left_paddle_instance_buffer: Buffer,
    right_paddle_instance_buffer: Buffer,
    ball_instance_buffer: Buffer,
    max_instances: usize,
    // Game state
    game_state: GameState,
    // Input state
    paddle_dir: i8, // -1 = up, 0 = stop, 1 = down
    // WebSocket (placeholder - actual implementation handled by JS)
    ws: Option<web_sys::WebSocket>,
}

#[wasm_bindgen]
pub struct WasmClient(Client);

#[wasm_bindgen]
impl WasmClient {
    /// Create a new WASM client instance
    #[wasm_bindgen(constructor)]
    pub async fn new(canvas: HtmlCanvasElement) -> Result<WasmClient, JsValue> {
        // Set up panic hook for better error messages
        console_error_panic_hook::set_once();

        // Log canvas info
        web_sys::console::log_1(
            &format!(
                "🎯 WasmClient::new called: canvas size={}x{}",
                canvas.width(),
                canvas.height()
            )
            .into(),
        );

        // Initialize wgpu
        let instance = wgpu::Instance::new(&InstanceDescriptor {
            backends: Backends::BROWSER_WEBGPU,
            ..Default::default()
        });

        web_sys::console::log_1(&"🔧 Creating WebGPU surface...".into());

        let canvas_target = canvas.clone();
        let surface = instance
            .create_surface(SurfaceTarget::Canvas(canvas_target))
            .map_err(|e| {
                web_sys::console::error_1(&format!("❌ Failed to create surface: {:?}", e).into());
                format!("Failed to create surface: {:?}", e)
            })?;

        web_sys::console::log_1(&"✅ Surface created".into());

        web_sys::console::log_1(&"🔧 Requesting adapter...".into());
        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or_else(|| {
                web_sys::console::error_1(&"❌ Failed to find adapter".into());
                "Failed to find adapter"
            })?;

        web_sys::console::log_1(&"✅ Adapter found".into());

        web_sys::console::log_1(&"🔧 Requesting device...".into());
        let (device, queue) = adapter
            .request_device(
                &DeviceDescriptor {
                    label: Some("Device"),
                    required_features: Features::empty(),
                    required_limits: Limits::downlevel_webgl2_defaults(),
                    memory_hints: MemoryHints::default(),
                },
                None,
            )
            .await
            .map_err(|e| {
                web_sys::console::error_1(&format!("❌ Failed to create device: {:?}", e).into());
                format!("Failed to create device: {:?}", e)
            })?;

        web_sys::console::log_1(&"✅ Device created".into());

        let width = canvas.width();
        let height = canvas.height();
        let size = (width, height);

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let surface_config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width,
            height,
            present_mode: PresentMode::Fifo,
            alpha_mode: CompositeAlphaMode::Auto,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &surface_config);

        // Create orthographic camera for 2D (32x24 arena)
        // Note: Canvas aspect ratio might differ, but we keep game coordinates fixed
        let camera = Camera::orthographic(32.0, 24.0);

        // Debug: Log camera setup
        web_sys::console::log_1(
            &format!(
                "📷 Camera: canvas=({}, {}), game=32x24, aspect={:.2}",
                width,
                height,
                width as f32 / height as f32
            )
            .into(),
        );

        // Create camera buffer (256 bytes for alignment)
        let camera_uniform = CameraUniform::from_camera(&camera);
        let camera_buffer = device.create_buffer_init(&util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        // Create camera bind group layout
        let camera_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Camera Bind Group Layout"),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        // Create camera bind group
        let camera_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Camera Bind Group"),
            layout: &camera_bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        // Create meshes
        let rectangle_mesh = create_rectangle(&device);
        let circle_mesh = create_circle(&device, 32); // 32 segments

        // Load shader
        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Shader"),
            source: ShaderSource::Wgsl(include_str!("../shaders/pong.wgsl").into()),
        });

        // Create render pipeline
        let render_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&camera_bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[
                    // Vertex buffer layout
                    VertexBufferLayout {
                        array_stride: 12, // 3 floats (position)
                        step_mode: VertexStepMode::Vertex,
                        attributes: &vertex_attr_array![0 => Float32x3],
                    },
                    // Instance buffer layout
                    VertexBufferLayout {
                        array_stride: 32, // 8 floats (transform + tint)
                        step_mode: VertexStepMode::Instance,
                        attributes: &[
                            // Location 1: transform (x, y, scale_x, scale_y)
                            VertexAttribute {
                                format: VertexFormat::Float32x4,
                                offset: 0,
                                shader_location: 1,
                            },
                            // Location 2: tint (r, g, b, a)
                            VertexAttribute {
                                format: VertexFormat::Float32x4,
                                offset: 16,
                                shader_location: 2,
                            },
                        ],
                    },
                ],
                compilation_options: Default::default(),
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(ColorTargetState {
                    format: surface_format,
                    blend: Some(BlendState::REPLACE),
                    write_mask: ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: None, // Disable culling to debug rendering
                polygon_mode: PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // Create instance buffers
        let max_instances = 32;
        // Each paddle buffer needs 1 instance
        let paddle_instance_buffer_size = std::mem::size_of::<InstanceData>() as u64;

        // Separate buffers for each paddle (non-instanced solution)
        let left_paddle_instance_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Left Paddle Instance Buffer"),
            size: paddle_instance_buffer_size,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let right_paddle_instance_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Right Paddle Instance Buffer"),
            size: paddle_instance_buffer_size,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Ball buffer needs 1 instance
        let ball_instance_buffer_size = std::mem::size_of::<InstanceData>() as u64;

        let ball_instance_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Ball Instance Buffer"),
            size: ball_instance_buffer_size,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let game_state = GameState::new();

        Ok(WasmClient(Client {
            device,
            queue,
            surface,
            surface_config,
            size,
            camera,
            camera_buffer,
            camera_bind_group,
            camera_bind_group_layout,
            rectangle_mesh,
            circle_mesh,
            render_pipeline,
            left_paddle_instance_buffer,
            right_paddle_instance_buffer,
            ball_instance_buffer,
            max_instances,
            game_state,
            paddle_dir: 0,
            ws: None,
        }))
    }

    /// Render a frame
    #[wasm_bindgen]
    pub fn render(&mut self) -> Result<(), JsValue> {
        // Log immediately to verify function is called
        web_sys::console::log_1(&"🎨 RENDER CALLED".into());

        let client = &mut self.0;

        // Debug: Log render call occasionally
        static mut FRAME_COUNT: u32 = 0;
        unsafe {
            FRAME_COUNT += 1;
            // Log first frame and every 60 frames
            if FRAME_COUNT == 1 || FRAME_COUNT % 60 == 0 {
                web_sys::console::log_1(
                    &format!(
                        "🎨 Render frame {}: ball=({:.1}, {:.1}), paddles=({:.1}, {:.1})",
                        FRAME_COUNT,
                        client.game_state.ball_x,
                        client.game_state.ball_y,
                        client.game_state.paddle_left_y,
                        client.game_state.paddle_right_y
                    )
                    .into(),
                );
            }
        }

        let output = client.surface.get_current_texture().map_err(|e| {
            web_sys::console::error_1(&format!("❌ Failed to get current texture: {:?}", e).into());
            format!("Failed to get current texture: {:?}", e)
        })?;

        let view = output
            .texture
            .create_view(&TextureViewDescriptor::default());

        let mut encoder = client
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        // Simple approach: Just create instance data directly (no dummy workaround)
        let left_paddle_instances = vec![InstanceData {
            transform: [1.5, client.game_state.paddle_left_y, 0.8, 4.0],
            tint: [1.0, 1.0, 1.0, 1.0], // White
        }];

        let right_paddle_instances = vec![InstanceData {
            transform: [30.5, client.game_state.paddle_right_y, 0.8, 4.0],
            tint: [1.0, 1.0, 1.0, 1.0], // White
        }];

        let ball_instances = vec![InstanceData {
            transform: [client.game_state.ball_x, client.game_state.ball_y, 0.5, 0.5],
            tint: [1.0, 1.0, 0.2, 1.0], // Yellowish
        }];

        // Upload instance data to separate buffers
        client.queue.write_buffer(
            &client.left_paddle_instance_buffer,
            0,
            bytemuck::cast_slice(&left_paddle_instances),
        );
        client.queue.write_buffer(
            &client.right_paddle_instance_buffer,
            0,
            bytemuck::cast_slice(&right_paddle_instances),
        );
        client.queue.write_buffer(
            &client.ball_instance_buffer,
            0,
            bytemuck::cast_slice(&ball_instances),
        );

        // Begin render pass
        {
            let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 1.0,
                        }),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            // Set viewport to match canvas size (important for proper rendering)
            render_pass.set_viewport(
                0.0,
                0.0,
                client.size.0 as f32,
                client.size.1 as f32,
                0.0,
                1.0,
            );

            render_pass.set_pipeline(&client.render_pipeline);
            render_pass.set_bind_group(0, &client.camera_bind_group, &[]);

            // Debug: Log instance data on first frame
            static mut LOGGED_INSTANCES: bool = false;
            unsafe {
                if !LOGGED_INSTANCES {
                    web_sys::console::log_1(
                        &format!(
                            "🔍 Left paddle: x={:.1}, y={:.1}, scale=({:.1}, {:.1}), tint=({:.1}, {:.1}, {:.1}, {:.1})",
                            left_paddle_instances[0].transform[0],
                            left_paddle_instances[0].transform[1],
                            left_paddle_instances[0].transform[2],
                            left_paddle_instances[0].transform[3],
                            left_paddle_instances[0].tint[0],
                            left_paddle_instances[0].tint[1],
                            left_paddle_instances[0].tint[2],
                            left_paddle_instances[0].tint[3],
                        )
                        .into(),
                    );
                    web_sys::console::log_1(
                        &format!(
                            "🔍 Right paddle: x={:.1}, y={:.1}, scale=({:.1}, {:.1})",
                            right_paddle_instances[0].transform[0],
                            right_paddle_instances[0].transform[1],
                            right_paddle_instances[0].transform[2],
                            right_paddle_instances[0].transform[3],
                        )
                        .into(),
                    );
                    web_sys::console::log_1(
                        &format!(
                            "🔍 Ball: x={:.1}, y={:.1}, scale=({:.1}, {:.1})",
                            ball_instances[0].transform[0],
                            ball_instances[0].transform[1],
                            ball_instances[0].transform[2],
                            ball_instances[0].transform[3],
                        )
                        .into(),
                    );
                    web_sys::console::log_1(
                        &format!(
                            "🔍 Rectangle mesh: {} indices",
                            client.rectangle_mesh.index_count
                        )
                        .into(),
                    );
                    LOGGED_INSTANCES = true;
                }
            }

            // Draw left paddle
            render_pass.set_vertex_buffer(0, client.rectangle_mesh.vertex_buffer.slice(..));
            render_pass.set_vertex_buffer(1, client.left_paddle_instance_buffer.slice(..));
            render_pass.set_index_buffer(
                client.rectangle_mesh.index_buffer.slice(..),
                IndexFormat::Uint16,
            );
            render_pass.draw_indexed(0..client.rectangle_mesh.index_count, 0, 0..1);

            // Draw right paddle
            render_pass.set_vertex_buffer(1, client.right_paddle_instance_buffer.slice(..));
            render_pass.draw_indexed(0..client.rectangle_mesh.index_count, 0, 0..1);

            // Draw ball (circle)
            render_pass.set_vertex_buffer(0, client.circle_mesh.vertex_buffer.slice(..));
            render_pass.set_vertex_buffer(1, client.ball_instance_buffer.slice(..));
            render_pass.set_index_buffer(
                client.circle_mesh.index_buffer.slice(..),
                IndexFormat::Uint16,
            );
            render_pass.draw_indexed(0..client.circle_mesh.index_count, 0, 0..1);
        }

        let command_buffer = encoder.finish();
        client.queue.submit(std::iter::once(command_buffer));

        // Present the frame
        output.present();

        // Log occasionally to verify present is being called
        static mut PRESENT_COUNT: u32 = 0;
        unsafe {
            PRESENT_COUNT += 1;
            if PRESENT_COUNT == 1 || PRESENT_COUNT % 60 == 0 {
                web_sys::console::log_1(
                    &format!("✅ Frame presented (frame {})", PRESENT_COUNT).into(),
                );
            }
        }

        Ok(())
    }

    /// Handle key down event
    #[wasm_bindgen]
    pub fn on_key_down(&mut self, event: KeyboardEvent) {
        let key = event.key();
        let old_dir = self.0.paddle_dir;
        self.0.paddle_dir = match key.as_str() {
            "ArrowUp" | "w" | "W" => -1,
            "ArrowDown" | "s" | "S" => 1,
            _ => self.0.paddle_dir,
        };

        if self.0.paddle_dir != old_dir {
            web_sys::console::log_1(
                &format!("⌨️  Key down: {} -> paddle_dir={}", key, self.0.paddle_dir).into(),
            );
        }

        // Send input to server
        self.send_input();
    }

    /// Handle key up event
    #[wasm_bindgen]
    pub fn on_key_up(&mut self, event: KeyboardEvent) {
        let key = event.key();
        match key.as_str() {
            "ArrowUp" | "w" | "W" | "ArrowDown" | "s" | "S" => {
                self.0.paddle_dir = 0;
                web_sys::console::log_1(&format!("⌨️  Key up: {} -> paddle_dir=0", key).into());
                self.send_input();
            }
            _ => {}
        }
    }

    /// Handle incoming WebSocket message
    #[wasm_bindgen]
    pub fn on_message(&mut self, data: &[u8]) -> Result<(), JsValue> {
        let msg =
            S2C::from_bytes(data).map_err(|e| format!("Failed to decode S2C message: {:?}", e))?;

        match msg {
            S2C::Welcome { player_id } => {
                self.0.game_state.my_player_id = Some(player_id);
                web_sys::console::log_1(
                    &format!(
                        "✅ Joined as player {} ({})",
                        player_id,
                        if player_id == 0 { "LEFT" } else { "RIGHT" }
                    )
                    .into(),
                );
            }
            S2C::GameState {
                tick,
                ball_x,
                ball_y,
                paddle_left_y,
                paddle_right_y,
                score_left,
                score_right,
                ..
            } => {
                self.0.game_state.ball_x = ball_x;
                self.0.game_state.ball_y = ball_y;
                self.0.game_state.paddle_left_y = paddle_left_y;
                self.0.game_state.paddle_right_y = paddle_right_y;
                self.0.game_state.score_left = score_left;
                self.0.game_state.score_right = score_right;

                if tick == 1 {
                    web_sys::console::log_1(&"🎮 Game started!".into());
                }
                if tick % 60 == 0 {
                    web_sys::console::log_1(
                        &format!(
                            "Game state: ball=({:.1}, {:.1}), paddles=({:.1}, {:.1})",
                            ball_x, ball_y, paddle_left_y, paddle_right_y
                        )
                        .into(),
                    );
                }
            }
            S2C::GameOver { winner } => {
                web_sys::console::log_1(&format!("🏆 Game over! Winner: player {}", winner).into());
            }
            S2C::Pong { .. } => {
                // Handle pong response
            }
        }

        Ok(())
    }

    /// Send input to server (called from JavaScript)
    fn send_input(&self) {
        // This will be handled by JavaScript interop
        // JS will call get_input_bytes() and send via WebSocket
    }

    /// Get Join message bytes
    #[wasm_bindgen]
    pub fn get_join_bytes(&self, code: &str) -> Result<Vec<u8>, JsValue> {
        if code.len() != 5 {
            return Err("Match code must be 5 characters".into());
        }

        let mut code_bytes = [0u8; 5];
        code_bytes.copy_from_slice(code.as_bytes());

        let msg = C2S::Join { code: code_bytes };
        msg.to_bytes()
            .map_err(|e| format!("Failed to encode join: {:?}", e).into())
    }

    /// Get current input as bytes for sending
    #[wasm_bindgen]
    pub fn get_input_bytes(&self) -> Result<Vec<u8>, JsValue> {
        let player_id = self.0.game_state.my_player_id.unwrap_or(0);
        let msg = C2S::Input {
            player_id,
            paddle_dir: self.0.paddle_dir,
        };
        msg.to_bytes()
            .map_err(|e| format!("Failed to encode input: {:?}", e).into())
    }

    /// Resize the surface
    #[wasm_bindgen]
    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.0.size = (width, height);
            self.0.surface_config.width = width;
            self.0.surface_config.height = height;
            self.0
                .surface
                .configure(&self.0.device, &self.0.surface_config);
        }
    }
}

// Export initialization function
#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
}
