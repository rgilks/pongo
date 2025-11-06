//! WebGPU client for ISO game
//!
//! Engine-free rendering using wgpu for WebGPU API
//!
//! Based on geno-1 implementation using wgpu 24.0 with "webgpu" feature
//! Note: Canvas variant is only available when compiling for wasm32 target

#![cfg(target_arch = "wasm32")]

mod camera;
mod mesh;

use camera::{Camera, CameraUniform};
use mesh::{create_cube, create_ground_quad, create_sphere, Mesh};
use proto::{dequantize_pos, dequantize_yaw, PlayerP, C2S, S2C};
use std::collections::HashMap;
use wasm_bindgen::prelude::*;
use web_sys::{HtmlCanvasElement, WebSocket};
use wgpu::*;

/// Game state data for a single player
#[derive(Debug, Clone)]
struct PlayerData {
    pos: [f32; 2],
    yaw: f32,
    hp: u8,
}

/// Game state data for a single bolt
#[derive(Debug, Clone)]
struct BoltData {
    pos: [f32; 2],
    radius: f32,
    level: u8,
}

/// Game state data for a single pickup
#[derive(Debug, Clone)]
struct PickupData {
    pos: [f32; 2],
    kind: u8,
}

/// Instance data for rendering (matches shader InstanceData)
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Instance {
    transform: [f32; 4], // x, y, scale, rotation
    tint: [f32; 4],      // rgba
}

/// Game state tracking
struct GameState {
    players: HashMap<u16, PlayerData>,
    bolts: HashMap<u16, BoltData>,
    pickups: HashMap<u16, PickupData>,
    last_snapshot_id: u32,
}

impl GameState {
    fn new() -> Self {
        Self {
            players: HashMap::new(),
            bolts: HashMap::new(),
            pickups: HashMap::new(),
            last_snapshot_id: 0,
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
    sphere_mesh: Mesh,
    cube_mesh: Mesh,
    ground_mesh: Mesh,
    // Render pipeline
    render_pipeline: RenderPipeline,
    // Light buffer (SSBO)
    light_buffer: Buffer,
    light_count_buffer: Buffer,
    light_bind_group: BindGroup,
    // Instance buffers
    player_instance_buffer: Buffer,
    bolt_instance_buffer: Buffer,
    max_instances: usize,
    // Network
    ws: Option<WebSocket>,
    player_id: Option<u16>,
    game_state: GameState,
    input_seq: u32,
}

impl Client {
    /// Initialize WebGPU client
    pub async fn new(canvas: HtmlCanvasElement) -> Result<Self, JsValue> {
        // Get window size
        let width = canvas.width();
        let height = canvas.height();

        // Create instance with web backend
        let wgpu_instance = wgpu::Instance::default();

        // Create surface from canvas
        // Based on geno-1: wgpu::SurfaceTarget::Canvas(canvas.clone())
        // The "webgpu" feature + wasm32 target enables SurfaceTarget::Canvas variant
        let surface = wgpu_instance
            .create_surface(wgpu::SurfaceTarget::Canvas(canvas.clone()))
            .map_err(|e| JsValue::from_str(&format!("Failed to create surface: {:?}", e)))?;

        // Request adapter
        let adapter = wgpu_instance
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

        // Create isometric camera
        let aspect = width as f32 / height.max(1) as f32;
        let camera = Camera::new(aspect);

        // Create camera uniform buffer (256-byte aligned)
        let camera_uniform = CameraUniform::from_camera(&camera);
        let camera_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Camera Uniform Buffer"),
            size: std::mem::size_of::<CameraUniform>() as u64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&camera_buffer, 0, bytemuck::bytes_of(&camera_uniform));

        // Create camera bind group layout
        let camera_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Camera Bind Group Layout"),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
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
        let (sphere_vertices, sphere_indices) = create_sphere(16);
        let sphere_mesh = Mesh::new(&device, &queue, &sphere_vertices, &sphere_indices);

        let (cube_vertices, cube_indices) = create_cube();
        let cube_mesh = Mesh::new(&device, &queue, &cube_vertices, &cube_indices);

        let (ground_vertices, ground_indices) = create_ground_quad(64.0); // Arena size
        let ground_mesh = Mesh::new(&device, &queue, &ground_vertices, &ground_indices);

        // Load shader
        let shader_source = include_str!("../shaders/basic.wgsl");
        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Basic Shader"),
            source: ShaderSource::Wgsl(shader_source.into()),
        });

        // Create light buffers (SSBO for up to 8 lights)
        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct Light {
            pos: [f32; 3],
            radius: f32,
            color: [f32; 3],
            intensity: f32,
        }

        let max_lights = 8u32;
        let light_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Light Buffer"),
            size: (max_lights as u64 * std::mem::size_of::<Light>() as u64),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let light_count_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Light Count Buffer"),
            size: std::mem::size_of::<u32>() as u64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&light_count_buffer, 0, bytemuck::bytes_of(&0u32));

        // Create bind group layout for lights
        let light_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Light Bind Group Layout"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        // Create light bind group
        let light_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Light Bind Group"),
            layout: &light_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 1,
                    resource: light_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: light_count_buffer.as_entire_binding(),
                },
            ],
        });

        // Create render pipeline layout
        let render_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&camera_bind_group_layout, &light_bind_group_layout],
            push_constant_ranges: &[],
        });

        // Create instance buffers (for players and bolts)
        let max_instances = 64; // Support up to 64 players/bolts
        let instance_buffer_size = (max_instances * std::mem::size_of::<Instance>()) as u64;

        let player_instance_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Player Instance Buffer"),
            size: instance_buffer_size,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bolt_instance_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Bolt Instance Buffer"),
            size: instance_buffer_size,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create vertex buffer layouts (vertex + instance data)
        let vertex_buffer_layout = VertexBufferLayout {
            array_stride: std::mem::size_of::<mesh::Vertex>() as u64,
            step_mode: VertexStepMode::Vertex,
            attributes: &[
                VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: VertexFormat::Float32x3,
                },
                VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as u64,
                    shader_location: 1,
                    format: VertexFormat::Float32x3,
                },
            ],
        };

        let instance_buffer_layout = VertexBufferLayout {
            array_stride: std::mem::size_of::<Instance>() as u64,
            step_mode: VertexStepMode::Instance,
            attributes: &[
                VertexAttribute {
                    offset: 0,
                    shader_location: 2,
                    format: VertexFormat::Float32x4, // transform
                },
                VertexAttribute {
                    offset: std::mem::size_of::<[f32; 4]>() as u64,
                    shader_location: 3,
                    format: VertexFormat::Float32x4, // tint
                },
            ],
        };

        // Create render pipeline
        let render_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[vertex_buffer_layout, instance_buffer_layout],
                compilation_options: PipelineCompilationOptions::default(),
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(ColorTargetState {
                    format: surface_format,
                    blend: Some(BlendState::REPLACE),
                    write_mask: ColorWrites::ALL,
                })],
                compilation_options: PipelineCompilationOptions::default(),
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: Some(Face::Back),
                unclipped_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Ok(Self {
            device,
            queue,
            surface,
            surface_config,
            size: (width, height),
            camera,
            camera_buffer,
            camera_bind_group,
            camera_bind_group_layout,
            sphere_mesh,
            cube_mesh,
            ground_mesh,
            render_pipeline,
            light_buffer,
            light_count_buffer,
            light_bind_group,
            player_instance_buffer,
            bolt_instance_buffer,
            max_instances,
            ws: None,
            player_id: None,
            game_state: GameState::new(),
            input_seq: 0,
        })
    }

    /// Resize the rendering surface
    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.size = (width, height);
            self.surface_config.width = width;
            self.surface_config.height = height;
            self.surface.configure(&self.device, &self.surface_config);

            // Update camera aspect ratio
            let aspect = width as f32 / height as f32;
            self.camera.set_aspect(aspect);

            // Update camera uniform buffer
            let camera_uniform = CameraUniform::from_camera(&self.camera);
            self.queue
                .write_buffer(&self.camera_buffer, 0, bytemuck::bytes_of(&camera_uniform));
        }
    }

    /// Update instance buffers from game state
    fn update_instance_buffers(&mut self) {
        // Update player instances
        let mut player_instances = Vec::new();
        for player in self.game_state.players.values() {
            player_instances.push(Instance {
                transform: [player.pos[0], player.pos[1], 0.6, player.yaw], // x, y, scale (player radius), rotation
                tint: [1.0, 0.5, 0.5, 1.0],                                 // Red tint for players
            });
        }
        if !player_instances.is_empty() {
            let instance_data = bytemuck::cast_slice(&player_instances);
            self.queue
                .write_buffer(&self.player_instance_buffer, 0, instance_data);
        }

        // Update bolt instances
        let mut bolt_instances = Vec::new();
        for bolt in self.game_state.bolts.values() {
            // Bolt color based on level (L1=blue, L2=cyan, L3=white)
            let (r, g, b) = match bolt.level {
                1 => (0.2, 0.5, 1.0),
                2 => (0.0, 1.0, 1.0),
                3 => (1.0, 1.0, 1.0),
                _ => (0.5, 0.5, 0.5),
            };
            bolt_instances.push(Instance {
                transform: [bolt.pos[0], bolt.pos[1], bolt.radius, 0.0], // x, y, scale (radius), no rotation
                tint: [r, g, b, 1.0],
            });
        }
        if !bolt_instances.is_empty() {
            let instance_data = bytemuck::cast_slice(&bolt_instances);
            self.queue
                .write_buffer(&self.bolt_instance_buffer, 0, instance_data);
        }
    }

    /// Render a frame
    pub fn render(&mut self) -> Result<(), JsValue> {
        // Update instance buffers from game state
        self.update_instance_buffers();

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
            let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color {
                            r: 0.1,
                            g: 0.1,
                            b: 0.15,
                            a: 1.0,
                        }),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            // Set render pipeline
            render_pass.set_pipeline(&self.render_pipeline);

            // Set bind groups
            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
            render_pass.set_bind_group(1, &self.light_bind_group, &[]);

            // Draw ground quad (non-instanced)
            render_pass.set_vertex_buffer(0, self.ground_mesh.vertex_buffer.slice(..));
            render_pass
                .set_index_buffer(self.ground_mesh.index_buffer.slice(..), IndexFormat::Uint16);
            render_pass.draw_indexed(0..self.ground_mesh.index_count, 0, 0..1);

            // Draw players as instanced spheres
            let player_count = self.game_state.players.len().min(self.max_instances);
            if player_count > 0 {
                render_pass.set_vertex_buffer(0, self.sphere_mesh.vertex_buffer.slice(..));
                render_pass.set_vertex_buffer(1, self.player_instance_buffer.slice(..));
                render_pass
                    .set_index_buffer(self.sphere_mesh.index_buffer.slice(..), IndexFormat::Uint16);
                render_pass.draw_indexed(
                    0..self.sphere_mesh.index_count,
                    0,
                    0..player_count as u32,
                );
            }

            // Draw bolts as instanced spheres
            let bolt_count = self.game_state.bolts.len().min(self.max_instances);
            if bolt_count > 0 {
                render_pass.set_vertex_buffer(0, self.sphere_mesh.vertex_buffer.slice(..));
                render_pass.set_vertex_buffer(1, self.bolt_instance_buffer.slice(..));
                render_pass
                    .set_index_buffer(self.sphere_mesh.index_buffer.slice(..), IndexFormat::Uint16);
                render_pass.draw_indexed(0..self.sphere_mesh.index_count, 0, 0..bolt_count as u32);
            }
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    /// Connect to server WebSocket
    pub fn connect_websocket(&mut self, url: &str) -> Result<(), JsValue> {
        let ws = WebSocket::new(url)
            .map_err(|e| JsValue::from_str(&format!("Failed to create WebSocket: {:?}", e)))?;

        ws.set_binary_type(web_sys::BinaryType::Arraybuffer);

        // Set up message handler - will be handled via JavaScript callback
        // The actual message handling will be done via handle_websocket_message() called from JS
        self.ws = Some(ws);
        Ok(())
    }

    /// Handle incoming S2C message
    pub fn handle_s2c_message(&mut self, bytes: &[u8]) -> Result<(), JsValue> {
        let msg = S2C::from_bytes(bytes)
            .map_err(|e| JsValue::from_str(&format!("Failed to parse S2C: {:?}", e)))?;

        match msg {
            S2C::Welcome {
                player_id,
                params_hash: _,
                map_rev: _,
            } => {
                self.player_id = Some(player_id);
            }
            S2C::Snapshot {
                id,
                tick: _,
                t_ms: _,
                last_seq_ack: _,
                players,
                bolts,
                pickups,
                hill_owner: _,
                hill_progress_u16: _,
            } => {
                self.update_game_state(id, players, bolts, pickups);
            }
            S2C::Eliminated { player_id } => {
                self.game_state.players.remove(&player_id);
            }
            S2C::Ended { standings: _ } => {
                // Game ended
            }
        }
        Ok(())
    }

    /// Update game state from snapshot
    fn update_game_state(
        &mut self,
        snapshot_id: u32,
        players: Vec<PlayerP>,
        bolts: Vec<proto::BoltP>,
        pickups: Vec<proto::PickupP>,
    ) {
        self.game_state.last_snapshot_id = snapshot_id;

        // Update players
        self.game_state.players.clear();
        for player in players {
            let pos = [
                dequantize_pos(player.pos_q[0]),
                dequantize_pos(player.pos_q[1]),
            ];
            let yaw = dequantize_yaw(player.yaw_q);
            self.game_state.players.insert(
                player.id,
                PlayerData {
                    pos,
                    yaw,
                    hp: player.hp,
                },
            );
        }

        // Update bolts
        self.game_state.bolts.clear();
        for bolt in bolts {
            let pos = [dequantize_pos(bolt.pos_q[0]), dequantize_pos(bolt.pos_q[1])];
            // Radius from quantized value (rad_q is u8, scale appropriately)
            let radius = bolt.rad_q as f32 / 100.0; // Approximate scaling
            self.game_state.bolts.insert(
                bolt.id,
                BoltData {
                    pos,
                    radius,
                    level: bolt.level,
                },
            );
        }

        // Update pickups
        self.game_state.pickups.clear();
        for pickup in pickups {
            let pos = [
                dequantize_pos(pickup.pos_q[0]),
                dequantize_pos(pickup.pos_q[1]),
            ];
            self.game_state.pickups.insert(
                pickup.id,
                PickupData {
                    pos,
                    kind: pickup.kind,
                },
            );
        }
    }

    /// Send input to server
    pub fn send_input(
        &mut self,
        thrust: f32,
        turn: f32,
        bolt: u8,
        shield: u8,
    ) -> Result<(), JsValue> {
        if let Some(ws) = &self.ws {
            if ws.ready_state() == WebSocket::OPEN {
                self.input_seq += 1;
                let t_ms = web_sys::window()
                    .and_then(|w| w.performance())
                    .map(|p| p.now() as u32)
                    .unwrap_or(0);

                let thrust_i8 = (thrust.clamp(-1.0, 1.0) * 127.0) as i8;
                let turn_i8 = (turn.clamp(-1.0, 1.0) * 127.0) as i8;

                let input_msg = C2S::Input {
                    seq: self.input_seq,
                    t_ms,
                    thrust_i8,
                    turn_i8,
                    bolt: bolt.min(3),
                    shield: shield.min(3),
                };

                let bytes = input_msg.to_bytes().map_err(|e| {
                    JsValue::from_str(&format!("Failed to serialize input: {:?}", e))
                })?;

                ws.send_with_u8_array(&bytes)
                    .map_err(|e| JsValue::from_str(&format!("Failed to send input: {:?}", e)))?;
            }
        }
        Ok(())
    }

    /// Send join message
    pub fn send_join(&mut self, code: &str, avatar: u8, name_id: u8) -> Result<(), JsValue> {
        if let Some(ws) = &self.ws {
            if ws.ready_state() == WebSocket::OPEN {
                let code_bytes = code.as_bytes();
                if code_bytes.len() != 5 {
                    return Err(JsValue::from_str("Match code must be 5 characters"));
                }
                let mut code_array = [0u8; 5];
                code_array.copy_from_slice(code_bytes);

                let join_msg = C2S::Join {
                    code: code_array,
                    avatar,
                    name_id,
                };

                let bytes = join_msg.to_bytes().map_err(|e| {
                    JsValue::from_str(&format!("Failed to serialize join: {:?}", e))
                })?;

                ws.send_with_u8_array(&bytes)
                    .map_err(|e| JsValue::from_str(&format!("Failed to send join: {:?}", e)))?;
            }
        }
        Ok(())
    }
}

// Global client storage for WASM bindings
static mut CLIENT: Option<Client> = None;

#[wasm_bindgen]
pub fn init_client(canvas: HtmlCanvasElement) -> js_sys::Promise {
    wasm_bindgen_futures::future_to_promise(async move {
        match Client::new(canvas).await {
            Ok(client) => {
                unsafe {
                    CLIENT = Some(client);
                }
                Ok(JsValue::UNDEFINED)
            }
            Err(e) => Err(e),
        }
    })
}

#[wasm_bindgen]
pub fn connect_websocket(url: &str) -> Result<(), JsValue> {
    unsafe {
        if let Some(ref mut client) = CLIENT {
            client.connect_websocket(url)
        } else {
            Err(JsValue::from_str("Client not initialized"))
        }
    }
}

#[wasm_bindgen]
pub fn send_join(code: &str, avatar: u8, name_id: u8) -> Result<(), JsValue> {
    unsafe {
        if let Some(ref mut client) = CLIENT {
            client.send_join(code, avatar, name_id)
        } else {
            Err(JsValue::from_str("Client not initialized"))
        }
    }
}

#[wasm_bindgen]
pub fn send_input(thrust: f32, turn: f32, bolt: u8, shield: u8) -> Result<(), JsValue> {
    unsafe {
        if let Some(ref mut client) = CLIENT {
            client.send_input(thrust, turn, bolt, shield)
        } else {
            Err(JsValue::from_str("Client not initialized"))
        }
    }
}

#[wasm_bindgen]
pub fn render_frame() -> Result<(), JsValue> {
    unsafe {
        if let Some(ref mut client) = CLIENT {
            client.render()
        } else {
            Err(JsValue::from_str("Client not initialized"))
        }
    }
}

#[wasm_bindgen]
pub fn handle_websocket_message(bytes: &[u8]) -> Result<(), JsValue> {
    unsafe {
        if let Some(ref mut client) = CLIENT {
            client.handle_s2c_message(bytes)
        } else {
            Err(JsValue::from_str("Client not initialized"))
        }
    }
}
