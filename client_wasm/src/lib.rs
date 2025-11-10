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
    paddle_instance_buffer: Buffer,
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

        // Initialize wgpu
        let instance = wgpu::Instance::new(&InstanceDescriptor {
            backends: Backends::BROWSER_WEBGPU,
            ..Default::default()
        });

        let canvas_target = canvas.clone();
        let surface = instance
            .create_surface(SurfaceTarget::Canvas(canvas_target))
            .map_err(|e| format!("Failed to create surface: {:?}", e))?;

        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or("Failed to find adapter")?;

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
            .map_err(|e| format!("Failed to create device: {:?}", e))?;

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
        let camera = Camera::orthographic(32.0, 24.0);

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
                        attributes: &vertex_attr_array![1 => Float32x4, 2 => Float32x4],
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
                cull_mode: Some(Face::Back),
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
        let instance_buffer_size = (max_instances * std::mem::size_of::<InstanceData>()) as u64;

        let paddle_instance_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Paddle Instance Buffer"),
            size: instance_buffer_size,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let ball_instance_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Ball Instance Buffer"),
            size: instance_buffer_size,
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
            paddle_instance_buffer,
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
        let client = &mut self.0;

        let output = client
            .surface
            .get_current_texture()
            .map_err(|e| format!("Failed to get current texture: {:?}", e))?;

        let view = output
            .texture
            .create_view(&TextureViewDescriptor::default());

        let mut encoder = client
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        // Prepare instances
        let paddle_instances = vec![
            // Left paddle (at x=1.5, width=0.8, height=4.0) - RED
            InstanceData {
                transform: [1.5, client.game_state.paddle_left_y, 0.8, 4.0],
                tint: [1.0, 0.2, 0.2, 1.0], // Reddish
            },
            // Right paddle (at x=30.5, width=0.8, height=4.0) - BLUE
            InstanceData {
                transform: [30.5, client.game_state.paddle_right_y, 0.8, 4.0],
                tint: [0.2, 0.2, 1.0, 1.0], // Bluish
            },
        ];

        // Debug: log paddle positions and instance data
        if (client.game_state.ball_x * 100.0) as u32 % 100 == 0 {
            web_sys::console::log_1(
                &format!(
                    "🎨 {} paddles: [0]=({}|{:.1}|{}|{}) [1]=({}|{:.1}|{}|{})",
                    paddle_instances.len(),
                    paddle_instances[0].transform[0],
                    paddle_instances[0].transform[1],
                    paddle_instances[0].transform[2],
                    paddle_instances[0].transform[3],
                    paddle_instances[1].transform[0],
                    paddle_instances[1].transform[1],
                    paddle_instances[1].transform[2],
                    paddle_instances[1].transform[3],
                )
                .into(),
            );
        }

        let ball_instances = vec![InstanceData {
            transform: [client.game_state.ball_x, client.game_state.ball_y, 0.5, 0.5],
            tint: [1.0, 1.0, 0.2, 1.0], // Yellowish
        }];

        // Upload instance data
        client.queue.write_buffer(
            &client.paddle_instance_buffer,
            0,
            bytemuck::cast_slice(&paddle_instances),
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

            render_pass.set_pipeline(&client.render_pipeline);
            render_pass.set_bind_group(0, &client.camera_bind_group, &[]);

            // Draw paddles (rectangles)
            render_pass.set_vertex_buffer(0, client.rectangle_mesh.vertex_buffer.slice(..));
            render_pass.set_vertex_buffer(1, client.paddle_instance_buffer.slice(..));
            render_pass.set_index_buffer(
                client.rectangle_mesh.index_buffer.slice(..),
                IndexFormat::Uint16,
            );
            render_pass.draw_indexed(
                0..client.rectangle_mesh.index_count,
                0,
                0..paddle_instances.len() as u32,
            );

            // Draw ball (circle)
            render_pass.set_vertex_buffer(0, client.circle_mesh.vertex_buffer.slice(..));
            render_pass.set_vertex_buffer(1, client.ball_instance_buffer.slice(..));
            render_pass.set_index_buffer(
                client.circle_mesh.index_buffer.slice(..),
                IndexFormat::Uint16,
            );
            render_pass.draw_indexed(
                0..client.circle_mesh.index_count,
                0,
                0..ball_instances.len() as u32,
            );
        }

        client.queue.submit(std::iter::once(encoder.finish()));
        output.present();

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
