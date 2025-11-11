//! WebGPU client for Pong game

#![cfg(target_arch = "wasm32")]

mod camera;
mod mesh;

use camera::{Camera, CameraUniform};
use mesh::{create_circle, create_rectangle, Mesh, Vertex};
use proto::{C2S, S2C};
use wasm_bindgen::prelude::*;
use web_sys::{HtmlCanvasElement, KeyboardEvent};
use wgpu::util::DeviceExt;
use wgpu::*;

/// Instance data for rendering (matches shader InstanceInput)
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
    // Game state
    game_state: GameState,
    // Input state
    paddle_dir: i8, // -1 = up, 0 = stop, 1 = down
}

#[wasm_bindgen]
pub struct WasmClient(Client);

#[wasm_bindgen]
impl WasmClient {
    #[wasm_bindgen(constructor)]
    pub async fn new(canvas: HtmlCanvasElement) -> Result<WasmClient, JsValue> {
        console_error_panic_hook::set_once();

        // Initialize wgpu
        let instance = Instance::new(&InstanceDescriptor {
            backends: Backends::BROWSER_WEBGPU,
            ..Default::default()
        });

        let canvas_clone = canvas.clone();
        let surface = instance
            .create_surface(SurfaceTarget::Canvas(canvas_clone))
            .map_err(|e| format!("Failed to create surface: {:?}", e))?;

        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or_else(|| "Failed to find adapter")?;

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
        let mut camera_uniform = CameraUniform::new();
        camera_uniform.update_view_proj(&camera);

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

        // Vertex buffer layout
        let vertex_buffer_layout = VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as u64,
            step_mode: VertexStepMode::Vertex,
            attributes: &[VertexAttribute {
                offset: 0,
                shader_location: 0,
                format: VertexFormat::Float32x3,
            }],
        };

        // Instance buffer layout
        let instance_buffer_layout = VertexBufferLayout {
            array_stride: std::mem::size_of::<InstanceData>() as u64,
            step_mode: VertexStepMode::Instance,
            attributes: &[
                VertexAttribute {
                    offset: 0,
                    shader_location: 1,
                    format: VertexFormat::Float32x4, // transform
                },
                VertexAttribute {
                    offset: std::mem::size_of::<[f32; 4]>() as u64,
                    shader_location: 2,
                    format: VertexFormat::Float32x4, // tint
                },
            ],
        };

        let render_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[vertex_buffer_layout, instance_buffer_layout],
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
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // Create instance buffers
        let instance_buffer_size = std::mem::size_of::<InstanceData>() as u64;

        let left_paddle_instance_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Left Paddle Instance Buffer"),
            size: instance_buffer_size,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let right_paddle_instance_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Right Paddle Instance Buffer"),
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
            game_state: GameState::new(),
            paddle_dir: 0,
        }))
    }

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

        // Update instance data
        // Game config: 32x24 arena, paddles at x=1.5 and 30.5
        let paddle_left_x = 1.5;
        let paddle_right_x = 30.5;
        let paddle_width = 0.8;
        let paddle_height = 4.0;
        let ball_radius = 0.5;

        let left_paddle_instance = InstanceData {
            transform: [
                paddle_left_x,
                client.game_state.paddle_left_y,
                paddle_width,
                paddle_height,
            ],
            tint: [1.0, 1.0, 1.0, 1.0], // White
        };

        let right_paddle_instance = InstanceData {
            transform: [
                paddle_right_x,
                client.game_state.paddle_right_y,
                paddle_width,
                paddle_height,
            ],
            tint: [1.0, 1.0, 1.0, 1.0], // White
        };

        let ball_instance = InstanceData {
            transform: [
                client.game_state.ball_x,
                client.game_state.ball_y,
                ball_radius * 2.0,
                ball_radius * 2.0,
            ],
            tint: [1.0, 1.0, 0.2, 1.0], // Yellowish
        };

        // Update instance buffers
        client.queue.write_buffer(
            &client.left_paddle_instance_buffer,
            0,
            bytemuck::cast_slice(&[left_paddle_instance]),
        );
        client.queue.write_buffer(
            &client.right_paddle_instance_buffer,
            0,
            bytemuck::cast_slice(&[right_paddle_instance]),
        );
        client.queue.write_buffer(
            &client.ball_instance_buffer,
            0,
            bytemuck::cast_slice(&[ball_instance]),
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

        client.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    /// Handle WebSocket message from server
    #[wasm_bindgen]
    pub fn on_message(&mut self, bytes: Vec<u8>) -> Result<(), JsValue> {
        let client = &mut self.0;

        let msg = S2C::from_bytes(&bytes)
            .map_err(|e| format!("Failed to deserialize message: {:?}", e))?;

        match msg {
            S2C::Welcome { player_id } => {
                client.game_state.my_player_id = Some(player_id);
                // Log player assignment
                // Note: console_log! macro not available in wasm, so we'll skip logging here
            }
            S2C::GameState {
                ball_x,
                ball_y,
                paddle_left_y,
                paddle_right_y,
                score_left,
                score_right,
                tick,
                ..
            } => {
                client.game_state.ball_x = ball_x;
                client.game_state.ball_y = ball_y;
                client.game_state.paddle_left_y = paddle_left_y;
                client.game_state.paddle_right_y = paddle_right_y;
                client.game_state.score_left = score_left;
                client.game_state.score_right = score_right;

                // Game state updated - rendering will show the changes
            }
            S2C::GameOver { winner } => {
                // Game over - winner determined
                // Could update UI here if needed
            }
            S2C::Pong { .. } => {
                // Handle pong
            }
        }

        Ok(())
    }

    /// Get join message bytes
    #[wasm_bindgen]
    pub fn get_join_bytes(&self, code: String) -> Vec<u8> {
        let code_bytes: Vec<u8> = code.bytes().take(5).collect();
        let mut code_array = [0u8; 5];
        code_array.copy_from_slice(&code_bytes[..5]);
        C2S::Join { code: code_array }
            .to_bytes()
            .unwrap_or_default()
    }

    /// Get input message bytes
    #[wasm_bindgen]
    pub fn get_input_bytes(&self) -> Vec<u8> {
        let player_id = self.0.game_state.my_player_id.unwrap_or(0);
        C2S::Input {
            player_id,
            paddle_dir: self.0.paddle_dir,
        }
        .to_bytes()
        .unwrap_or_default()
    }

    /// Handle key down event
    #[wasm_bindgen]
    pub fn on_key_down(&mut self, event: KeyboardEvent) {
        let key = event.key();
        self.0.paddle_dir = match key.as_str() {
            "ArrowUp" | "w" | "W" => -1,
            "ArrowDown" | "s" | "S" => 1,
            _ => self.0.paddle_dir,
        };
    }

    /// Handle key up event
    #[wasm_bindgen]
    pub fn on_key_up(&mut self, event: KeyboardEvent) {
        let key = event.key();
        match key.as_str() {
            "ArrowUp" | "w" | "W" | "ArrowDown" | "s" | "S" => {
                self.0.paddle_dir = 0;
            }
            _ => {}
        }
    }
}
