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
use game_core::*;
use hecs::World;
use mesh::{create_cube, create_ground_quad, create_sphere, Mesh};
use proto::{dequantize_pos, dequantize_yaw, PlayerP, C2S, S2C};
use std::collections::HashMap;
use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;
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
    pickup_instance_buffer: Buffer,
    block_instance_buffer: Buffer,
    dummy_instance_buffer: Buffer, // For non-instanced draws (ground)
    max_instances: usize,
    // Network (WebSocket managed in JavaScript)
    player_id: Option<u16>,
    game_state: GameState, // Server state (for reconciliation)
    input_seq: u32,

    // Client prediction: local simulation
    local_world: World,
    local_time: Time,
    local_map: GameMap,
    local_rng: GameRng,
    local_score: Score,
    local_events: Events,
    local_config: Config,
    local_net_queue: NetQueue,

    // Pending inputs (for reconciliation)
    pending_inputs: Vec<(u32, InputEvent)>, // (seq, input) - stored until acked
    last_acked_seq: u32,                    // Last input sequence acked by server
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

        let pickup_instance_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Pickup Instance Buffer"),
            size: instance_buffer_size,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Block instance buffer (for walls/obstacles - test map has ~8 blocks)
        let max_blocks = 16;
        let block_instance_buffer_size = (max_blocks * std::mem::size_of::<Instance>()) as u64;
        let block_instance_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Block Instance Buffer"),
            size: block_instance_buffer_size,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Initialize block instances with test map data
        // Test map: 4 outer walls + 4 interior obstacles
        // Convert AABBs to center + average size (shader uses uniform scaling)
        let mut block_instances = Vec::new();
        let arena_size = 20.0;
        let wall_thickness = 1.0;

        // Outer walls (4 walls) - convert AABBs to center + average size
        // South wall: min=(-20,-20), max=(20,-19) -> center=(0,-19.5), size=(40,1) -> avg=20.5
        block_instances.push(Instance {
            transform: [0.0, -arena_size + wall_thickness * 0.5, 20.5, 0.0],
            tint: [0.4, 0.4, 0.5, 1.0], // Gray walls
        });
        // North wall: min=(-20,19), max=(20,20) -> center=(0,19.5), size=(40,1) -> avg=20.5
        block_instances.push(Instance {
            transform: [0.0, arena_size - wall_thickness * 0.5, 20.5, 0.0],
            tint: [0.4, 0.4, 0.5, 1.0],
        });
        // West wall: min=(-20,-20), max=(-19,20) -> center=(-19.5,0), size=(1,40) -> avg=20.5
        block_instances.push(Instance {
            transform: [-arena_size + wall_thickness * 0.5, 0.0, 20.5, 0.0],
            tint: [0.4, 0.4, 0.5, 1.0],
        });
        // East wall: min=(19,-20), max=(20,20) -> center=(19.5,0), size=(1,40) -> avg=20.5
        block_instances.push(Instance {
            transform: [arena_size - wall_thickness * 0.5, 0.0, 20.5, 0.0],
            tint: [0.4, 0.4, 0.5, 1.0],
        });

        // Interior obstacles (4 blocks, 2x2 each) - already center + size
        for (x, y) in [(-8.0, 0.0), (8.0, 0.0), (0.0, -8.0), (0.0, 8.0)] {
            block_instances.push(Instance {
                transform: [x, y, 2.0, 0.0], // x, y, scale (size), rotation
                tint: [0.5, 0.3, 0.2, 1.0],  // Brown obstacles
            });
        }

        if !block_instances.is_empty() {
            let block_data = bytemuck::cast_slice(&block_instances);
            queue.write_buffer(&block_instance_buffer, 0, block_data);
        }

        // Create dummy instance buffer for non-instanced draws (ground)
        let dummy_instance = Instance {
            transform: [0.0, 0.0, 1.0, 0.0], // identity transform
            tint: [0.2, 0.25, 0.3, 1.0],     // Dark blue-gray ground
        };
        let dummy_instance_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Dummy Instance Buffer"),
            size: std::mem::size_of::<Instance>() as u64,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(
            &dummy_instance_buffer,
            0,
            bytemuck::bytes_of(&dummy_instance),
        );

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
            pickup_instance_buffer,
            block_instance_buffer,
            dummy_instance_buffer,
            max_instances,
            player_id: None,
            game_state: GameState::new(),
            input_seq: 0,
            // Initialize local simulation
            local_world: World::new(),
            local_time: Time::new(),
            local_map: GameMap::new(Map::test_map()),
            local_rng: GameRng::new(),
            local_score: Score::new(),
            local_events: Events::new(),
            local_config: Config::new(),
            local_net_queue: NetQueue::new(),
            pending_inputs: Vec::new(),
            last_acked_seq: 0,
        })
    }

    /// Update camera uniform buffer
    fn update_camera_buffer(&mut self) {
        // Make camera follow the player if we have one (use local simulation for smooth following)
        if let Some(player_id) = self.player_id {
            // Try to get player from local world first (client prediction)
            let mut found = false;
            for (_, (player, transform)) in
                self.local_world.query::<(&Player, &Transform2D)>().iter()
            {
                if player.id == player_id {
                    self.camera
                        .set_target(glam::Vec3::new(transform.pos.x, 0.0, transform.pos.y));
                    found = true;
                    break;
                }
            }

            // Fallback to server state if not in local world
            if !found {
                if let Some(player_data) = self.game_state.players.get(&player_id) {
                    self.camera.set_target(glam::Vec3::new(
                        player_data.pos[0],
                        0.0,
                        player_data.pos[1],
                    ));
                }
            }
        }

        let camera_uniform = CameraUniform::from_camera(&self.camera);
        self.queue
            .write_buffer(&self.camera_buffer, 0, bytemuck::bytes_of(&camera_uniform));
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
            self.update_camera_buffer();
        }
    }

    /// Extract render data from local world (for client prediction)
    fn extract_render_data_from_world(&self) -> (Vec<PlayerData>, Vec<BoltData>, Vec<PickupData>) {
        let mut players = Vec::new();
        let mut bolts = Vec::new();
        let mut pickups = Vec::new();

        // Extract players
        for (_, (player, transform, health)) in self
            .local_world
            .query::<(&Player, &Transform2D, &Health)>()
            .iter()
        {
            players.push(PlayerData {
                pos: [transform.pos.x, transform.pos.y],
                yaw: transform.yaw,
                hp: 3 - health.damage.min(3), // Convert damage to HP
            });
        }

        // Extract bolts
        for (_, (bolt, transform)) in self.local_world.query::<(&Bolt, &Transform2D)>().iter() {
            bolts.push(BoltData {
                pos: [transform.pos.x, transform.pos.y],
                radius: bolt.radius,
                level: bolt.level,
            });
        }

        // Extract pickups
        for (_, (pickup, transform)) in self.local_world.query::<(&Pickup, &Transform2D)>().iter() {
            pickups.push(PickupData {
                pos: [transform.pos.x, transform.pos.y],
                kind: match pickup.kind {
                    game_core::components::PickupKind::Health => 0,
                    game_core::components::PickupKind::BoltUpgrade => 1,
                    game_core::components::PickupKind::ShieldModule => 2,
                },
            });
        }

        (players, bolts, pickups)
    }

    /// Update instance buffers from server state (temporary fallback)
    fn update_instance_buffers_from_server_state(&mut self) {
        // Update player instances from server state
        let mut player_instances = Vec::new();
        for player in self.game_state.players.values() {
            // Vibrant player colors - bright orange/red with slight glow
            player_instances.push(Instance {
                transform: [player.pos[0], player.pos[1], 0.6, player.yaw], // x, y, scale (player radius), rotation
                tint: [1.0, 0.4, 0.2, 1.0], // Bright orange-red for players
            });
        }
        if !player_instances.is_empty() {
            let instance_data = bytemuck::cast_slice(&player_instances);
            self.queue
                .write_buffer(&self.player_instance_buffer, 0, instance_data);
        }

        // Update bolt instances from server state
        let mut bolt_instances = Vec::new();
        for bolt in self.game_state.bolts.values() {
            // Bright, glowing bolt colors based on level
            let (r, g, b) = match bolt.level {
                1 => (0.3, 0.7, 1.0), // Bright blue
                2 => (0.0, 1.0, 1.0), // Cyan
                3 => (1.0, 0.9, 0.3), // Bright yellow-white
                _ => (0.8, 0.8, 0.8),
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

        // Update pickup instances from server state
        let mut pickup_instances = Vec::new();
        for pickup in self.game_state.pickups.values() {
            // Bright, vibrant pickup colors with glow
            let (r, g, b) = match pickup.kind {
                0 => (1.0, 0.3, 0.3), // Health - bright red
                1 => (0.4, 0.7, 1.0), // BoltUpgrade - bright blue
                2 => (0.3, 1.0, 0.4), // ShieldModule - bright green
                _ => (0.9, 0.9, 0.9), // Default - bright white
            };
            pickup_instances.push(Instance {
                transform: [pickup.pos[0], pickup.pos[1], 0.4, 0.0], // x, y, scale (radius), no rotation
                tint: [r, g, b, 1.0],
            });
        }
        if !pickup_instances.is_empty() {
            let instance_data = bytemuck::cast_slice(&pickup_instances);
            self.queue
                .write_buffer(&self.pickup_instance_buffer, 0, instance_data);
        }
    }

    /// Render a frame
    pub fn render(&mut self) -> Result<(), JsValue> {
        // TEMPORARILY DISABLED: Client prediction causing jerky behavior
        // TODO: Fix reconciliation to prevent jumps when syncing from server
        // For now, just render from server state (game_state) instead of local simulation

        // Update camera uniform buffer
        self.update_camera_buffer();

        // Update instance buffers from server state (temporarily, until client prediction is fixed)
        self.update_instance_buffers_from_server_state();

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
                            r: 0.05,
                            g: 0.08,
                            b: 0.12,
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

            // Draw ground quad (with dummy instance for pipeline compatibility)
            render_pass.set_vertex_buffer(0, self.ground_mesh.vertex_buffer.slice(..));
            render_pass.set_vertex_buffer(1, self.dummy_instance_buffer.slice(..));
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

            // Draw pickups as instanced spheres
            let pickup_count = self.game_state.pickups.len().min(self.max_instances);
            if pickup_count > 0 {
                render_pass.set_vertex_buffer(0, self.sphere_mesh.vertex_buffer.slice(..));
                render_pass.set_vertex_buffer(1, self.pickup_instance_buffer.slice(..));
                render_pass
                    .set_index_buffer(self.sphere_mesh.index_buffer.slice(..), IndexFormat::Uint16);
                render_pass.draw_indexed(
                    0..self.sphere_mesh.index_count,
                    0,
                    0..pickup_count as u32,
                );
            }

            // Draw blocks as instanced cubes (8 blocks: 4 walls + 4 obstacles)
            let block_count = 8u32;
            render_pass.set_vertex_buffer(0, self.cube_mesh.vertex_buffer.slice(..));
            render_pass.set_vertex_buffer(1, self.block_instance_buffer.slice(..));
            render_pass
                .set_index_buffer(self.cube_mesh.index_buffer.slice(..), IndexFormat::Uint16);
            render_pass.draw_indexed(0..self.cube_mesh.index_count, 0, 0..block_count);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    /// Connect to server WebSocket (called from JS, WebSocket managed in JS)
    pub fn connect_websocket(&mut self, _url: &str) -> Result<(), JsValue> {
        // WebSocket is created and managed in JavaScript
        // This function just marks that we're ready to receive messages
        // The actual WebSocket connection is handled in the HTML/JS code
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
                // Welcome message received - player_id assigned
                self.player_id = Some(player_id);

                // Initialize local world with player (will be synced from first snapshot)
                // For now, wait for first snapshot to initialize
            }
            S2C::Snapshot {
                id,
                tick: _,
                t_ms: _,
                last_seq_ack,
                players,
                bolts,
                pickups,
                hill_owner: _,
                hill_progress_u16: _,
            } => {
                // Snapshot received - reconcile with local simulation
                self.reconcile(last_seq_ack);

                // Sync local world from server snapshot
                self.sync_local_world_from_snapshot(&players, &bolts, &pickups);

                // Replay unacked inputs after syncing
                // Clear net_queue first, then queue unacked inputs
                // They will be applied in the next render frame
                self.local_net_queue.inputs.clear();
                for (seq, input) in &self.pending_inputs {
                    if *seq > last_seq_ack {
                        self.local_net_queue.inputs.push(input.clone());
                    }
                }

                // Don't step here - let the render loop step at consistent rate
                // The sync already updated positions from server, so we're in sync

                // Also update server state for reference
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

    /// Step local simulation (client prediction)
    fn step_local_simulation(&mut self, dt: f32) {
        // Set dt for this step (step() will update time.now internally)
        self.local_time.dt = dt;

        // Run game simulation step
        // Note: step() updates time.now internally, so we don't need to do it here
        step(
            &mut self.local_world,
            &mut self.local_time,
            &self.local_map,
            &mut self.local_rng,
            &mut self.local_score,
            &mut self.local_events,
            &self.local_config,
            &mut self.local_net_queue,
        );
    }

    /// Sync local world from server snapshot (for reconciliation)
    fn sync_local_world_from_snapshot(
        &mut self,
        players: &[PlayerP],
        bolts: &[proto::BoltP],
        pickups: &[proto::PickupP],
    ) {
        // Sync players: update existing or create new
        for player in players {
            let pos = glam::Vec2::new(
                dequantize_pos(player.pos_q[0]),
                dequantize_pos(player.pos_q[1]),
            );
            let yaw = dequantize_yaw(player.yaw_q);

            // Find or create player entity
            let mut player_entity = None;
            for (entity, p) in self.local_world.query::<&Player>().iter() {
                if p.id == player.id {
                    player_entity = Some(entity);
                    break;
                }
            }

            if let Some(entity) = player_entity {
                // Update existing player
                for (e, transform) in self.local_world.query_mut::<&mut Transform2D>() {
                    if e == entity {
                        transform.pos = pos;
                        transform.yaw = yaw;
                        break;
                    }
                }
                for (e, health) in self.local_world.query_mut::<&mut Health>() {
                    if e == entity {
                        health.damage = 3 - player.hp.min(3);
                        break;
                    }
                }
            } else {
                // Create new player
                create_player(&mut self.local_world, player.id, 0, 0, pos);
                // Update position/yaw after creation
                // Find entity first
                let mut target_entity = None;
                for (e, p) in self.local_world.query::<&Player>().iter() {
                    if p.id == player.id {
                        target_entity = Some(e);
                        break;
                    }
                }
                // Then update components
                if let Some(entity) = target_entity {
                    for (e, transform) in self.local_world.query_mut::<&mut Transform2D>() {
                        if e == entity {
                            transform.pos = pos;
                            transform.yaw = yaw;
                            break;
                        }
                    }
                    for (e, health) in self.local_world.query_mut::<&mut Health>() {
                        if e == entity {
                            health.damage = 3 - player.hp.min(3);
                            break;
                        }
                    }
                }
            }
        }

        // Sync bolts: remove old ones, let local simulation create new ones
        // (Bolts are created by fire_bolts system, so we just sync positions)
        // For now, we'll let the local simulation handle bolts naturally

        // Sync pickups: managed by spawn pads, let game systems handle them
        // Pickups will spawn naturally from spawn pads
    }

    /// Reconcile local simulation with server snapshot
    fn reconcile(&mut self, last_seq_ack: u32) {
        // Update last acked sequence
        self.last_acked_seq = last_seq_ack;

        // Remove acked inputs from pending list
        self.pending_inputs.retain(|(seq, _)| *seq > last_seq_ack);

        // If we have unacked inputs, we need to rewind and replay
        // For now, we'll just sync from server state and replay pending inputs
        // In a full implementation, we'd save/restore world state
    }

    /// Prepare input message bytes (JavaScript will send via WebSocket)
    /// Also applies input immediately to local simulation (client prediction)
    pub fn prepare_input(
        &mut self,
        thrust: f32,
        turn: f32,
        bolt: u8,
        shield: u8,
    ) -> Result<Vec<u8>, JsValue> {
        self.input_seq += 1;
        let t_ms = web_sys::window()
            .and_then(|w| w.performance())
            .map(|p| p.now() as u32)
            .unwrap_or(0);

        let thrust_i8 = (thrust.clamp(-1.0, 1.0) * 127.0) as i8;
        let turn_i8 = (turn.clamp(-1.0, 1.0) * 127.0) as i8;

        // Apply input immediately to local simulation (client prediction)
        if let Some(player_id) = self.player_id {
            // Only apply if local world is initialized (has players)
            let has_players = self.local_world.query::<&Player>().iter().next().is_some();
            if has_players {
                // Create input event
                let input_event = InputEvent {
                    player_id,
                    seq: self.input_seq,
                    t_ms,
                    thrust,
                    turn,
                    bolt_level: bolt.min(3),
                    shield_level: shield.min(3),
                };

                // Store for reconciliation
                self.pending_inputs
                    .push((self.input_seq, input_event.clone()));

                // Queue input for next simulation step (don't step here, let render loop handle it)
                self.local_net_queue.inputs.push(input_event);
            }
        }

        let input_msg = C2S::Input {
            seq: self.input_seq,
            t_ms,
            thrust_i8,
            turn_i8,
            bolt: bolt.min(3),
            shield: shield.min(3),
        };

        input_msg
            .to_bytes()
            .map_err(|e| JsValue::from_str(&format!("Failed to serialize input: {:?}", e)))
    }

    /// Prepare join message bytes (JavaScript will send via WebSocket)
    pub fn prepare_join(
        &mut self,
        code: &str,
        avatar: u8,
        name_id: u8,
    ) -> Result<Vec<u8>, JsValue> {
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

        join_msg
            .to_bytes()
            .map_err(|e| JsValue::from_str(&format!("Failed to serialize join: {:?}", e)))
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
pub fn prepare_join(code: &str, avatar: u8, name_id: u8) -> Result<Vec<u8>, JsValue> {
    unsafe {
        if let Some(ref mut client) = CLIENT {
            client.prepare_join(code, avatar, name_id)
        } else {
            Err(JsValue::from_str("Client not initialized"))
        }
    }
}

#[wasm_bindgen]
pub fn prepare_input(thrust: f32, turn: f32, bolt: u8, shield: u8) -> Result<Vec<u8>, JsValue> {
    unsafe {
        if let Some(ref mut client) = CLIENT {
            client.prepare_input(thrust, turn, bolt, shield)
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
