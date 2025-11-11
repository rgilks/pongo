//! WebGPU client for Pong game

#![cfg(target_arch = "wasm32")]

mod camera;
mod input;
mod mesh;
mod network;
mod state;

use camera::{Camera, CameraUniform};
use game_core::{
    create_ball, create_paddle, step, Ball, Config, Events, GameMap, GameRng, NetQueue, Paddle,
    RespawnState, Score, Time,
};
use hecs::World;
use mesh::{create_circle, create_rectangle, Mesh, Vertex};
use network::handle_message;
use proto::S2C;
use state::GameState;
use wasm_bindgen::prelude::*;
use web_sys::{window, HtmlCanvasElement, KeyboardEvent};
use wgpu::util::DeviceExt;
use wgpu::*;

/// Instance data for rendering (matches shader InstanceInput)
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct InstanceData {
    transform: [f32; 4], // x, y, scale_x, scale_y
    tint: [f32; 4],      // rgba
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
    // Trail effect using ping-pong textures
    trail_texture_a: Texture,
    trail_texture_b: Texture,
    trail_texture_view_a: TextureView,
    trail_texture_view_b: TextureView,
    trail_sampler: Sampler,
    trail_bind_group_a: BindGroup,
    trail_bind_group_b: BindGroup,
    trail_bind_group_layout: BindGroupLayout,
    trail_render_pipeline: RenderPipeline,
    trail_vertex_buffer: Buffer,
    trail_use_a: bool, // Toggle between A and B for ping-pong
    // Game state
    game_state: GameState,
    // Input state
    paddle_dir: i8, // -1 = up, 0 = stop, 1 = down
    // Frame timing for interpolation (using performance.now() for better precision)
    last_frame_time: f64,        // Last render frame time (in milliseconds)
    last_sim_time: f64,          // Last simulation step time (in milliseconds)
    sim_accumulator: f32,        // Accumulated time for fixed timestep simulation
    last_prediction_time: f64,   // Last prediction step time (in milliseconds)
    prediction_accumulator: f32, // Accumulated time for fixed timestep prediction
    // Performance metrics
    fps: f32,
    fps_frame_count: u32,
    fps_last_update: f64,
    ping_ms: f32,              // Current ping in milliseconds
    ping_pending: Option<f64>, // Timestamp when ping was sent (in milliseconds since epoch)
    update_display_ms: f32,    // Throttled update delay for display (updates slower)
    update_last_display: f64,  // Last time we updated the display value
    // Rendering optimizations
    enable_trails: bool, // Enable/disable trail effect for performance
    last_instance_data: Option<(InstanceData, InstanceData, InstanceData)>, // Cache to avoid unnecessary buffer writes
    // Local game mode (AI opponent)
    is_local_game: bool,
    local_world: Option<World>,
    local_time: Option<Time>,
    local_map: Option<GameMap>,
    local_config: Option<Config>,
    local_score: Option<Score>,
    local_events: Option<Events>,
    local_net_queue: Option<NetQueue>,
    local_rng: Option<GameRng>,
    local_respawn_state: Option<RespawnState>,
    // Client prediction state (for online games)
    input_seq: u32,                                // Next input sequence number
    predicted_world: Option<World>,                // Local predicted world state
    predicted_time: Option<Time>,                  // Predicted time state
    predicted_map: Option<GameMap>,                // Game map (same for all clients)
    predicted_config: Option<Config>,              // Game config (same for all clients)
    predicted_score: Option<Score>,                // Predicted score
    predicted_events: Option<Events>,              // Predicted events
    predicted_net_queue: Option<NetQueue>,         // Input queue for prediction
    predicted_rng: Option<GameRng>,                // RNG for prediction (seeded from server)
    predicted_respawn_state: Option<RespawnState>, // Predicted respawn state
    last_reconciled_tick: u32,                     // Last server tick we reconciled to
    predicted_tick: u32,                           // Current predicted tick
    input_history: Vec<(u32, i8)>,                 // History of (seq, paddle_dir) inputs for replay
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
            .unwrap_or_else(|| {
                surface_caps
                    .formats
                    .first()
                    .copied()
                    .expect("No surface formats available")
            });

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

        // Create ping-pong trail textures for accumulation effect
        let trail_texture_a = device.create_texture(&TextureDescriptor {
            label: Some("Trail Texture A"),
            size: Extent3d {
                width: surface_config.width,
                height: surface_config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: surface_format,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let trail_texture_b = device.create_texture(&TextureDescriptor {
            label: Some("Trail Texture B"),
            size: Extent3d {
                width: surface_config.width,
                height: surface_config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: surface_format,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let trail_texture_view_a = trail_texture_a.create_view(&TextureViewDescriptor::default());
        let trail_texture_view_b = trail_texture_b.create_view(&TextureViewDescriptor::default());
        let trail_sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("Trail Sampler"),
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
            ..Default::default()
        });

        // Create bind group layout for trail shader
        let trail_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Trail Bind Group Layout"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        multisampled: false,
                        view_dimension: TextureViewDimension::D2,
                        sample_type: TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let trail_bind_group_a = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Trail Bind Group A"),
            layout: &trail_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&trail_texture_view_a),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&trail_sampler),
                },
            ],
        });

        let trail_bind_group_b = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Trail Bind Group B"),
            layout: &trail_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&trail_texture_view_b),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&trail_sampler),
                },
            ],
        });

        // Create fullscreen quad for trail shader
        let trail_vertices: [f32; 16] = [
            -1.0, -1.0, 0.0, 1.0, 1.0, -1.0, 1.0, 1.0, -1.0, 1.0, 0.0, 0.0, 1.0, 1.0, 1.0, 0.0,
        ];
        let trail_vertex_buffer = device.create_buffer_init(&util::BufferInitDescriptor {
            label: Some("Trail Vertex Buffer"),
            contents: bytemuck::cast_slice(&trail_vertices),
            usage: BufferUsages::VERTEX,
        });

        // Create trail render pipeline (blends previous frame with fade)
        let trail_shader_code = r#"
            struct VertexInput {
                @location(0) position: vec2<f32>,
                @location(1) tex_coord: vec2<f32>,
            }

            struct VertexOutput {
                @builtin(position) clip_position: vec4<f32>,
                @location(0) tex_coord: vec2<f32>,
            }

            @vertex
            fn vs_main(in: VertexInput) -> VertexOutput {
                var out: VertexOutput;
                out.clip_position = vec4<f32>(in.position, 0.0, 1.0);
                out.tex_coord = in.tex_coord;
                return out;
            }

            @group(0) @binding(0) var trail_texture: texture_2d<f32>;
            @group(0) @binding(1) var trail_sampler: sampler;

            @fragment
            fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
                let color = textureSample(trail_texture, trail_sampler, in.tex_coord);
                // Fade the trail (multiply by 0.92 for nice trail persistence)
                // Lower value = faster fade, higher = longer trail
                return color * 0.92;
            }
        "#;

        let trail_shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Trail Shader"),
            source: ShaderSource::Wgsl(trail_shader_code.into()),
        });

        let trail_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Trail Pipeline Layout"),
            bind_group_layouts: &[&trail_bind_group_layout],
            push_constant_ranges: &[],
        });

        let trail_render_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Trail Render Pipeline"),
            layout: Some(&trail_pipeline_layout),
            vertex: VertexState {
                module: &trail_shader,
                entry_point: Some("vs_main"),
                buffers: &[VertexBufferLayout {
                    array_stride: 16, // 4 floats * 4 bytes
                    step_mode: VertexStepMode::Vertex,
                    attributes: &[
                        VertexAttribute {
                            offset: 0,
                            shader_location: 0,
                            format: VertexFormat::Float32x2,
                        },
                        VertexAttribute {
                            offset: 8,
                            shader_location: 1,
                            format: VertexFormat::Float32x2,
                        },
                    ],
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(FragmentState {
                module: &trail_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(ColorTargetState {
                    format: surface_format,
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleStrip,
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
            trail_texture_a,
            trail_texture_b,
            trail_texture_view_a,
            trail_texture_view_b,
            trail_sampler,
            trail_bind_group_a,
            trail_bind_group_b,
            trail_bind_group_layout,
            trail_render_pipeline,
            trail_vertex_buffer,
            trail_use_a: true,
            game_state: GameState::new(),
            paddle_dir: 0,
            last_frame_time: 0.0,
            last_sim_time: 0.0,
            sim_accumulator: 0.0,
            last_prediction_time: 0.0,
            prediction_accumulator: 0.0,
            fps: 0.0,
            fps_frame_count: 0,
            fps_last_update: 0.0,
            ping_ms: 0.0,
            ping_pending: None,
            update_display_ms: 0.0,
            update_last_display: 0.0,
            enable_trails: true, // Enable by default, can be toggled for performance
            last_instance_data: None,
            is_local_game: false,
            local_world: None,
            local_time: None,
            local_map: None,
            local_config: None,
            local_score: None,
            local_events: None,
            local_net_queue: None,
            local_rng: None,
            local_respawn_state: None,
            // Client prediction state
            input_seq: 0,
            predicted_world: None,
            predicted_time: None,
            predicted_map: None,
            predicted_config: None,
            predicted_score: None,
            predicted_events: None,
            predicted_net_queue: None,
            predicted_rng: None,
            predicted_respawn_state: None,
            last_reconciled_tick: 0,
            predicted_tick: 0,
            input_history: Vec::new(),
        }))
    }

    /// Get high-precision timestamp using performance.now() (faster than Date.now())
    fn performance_now() -> f64 {
        // Use js_sys::Reflect to access window.performance.now()
        window()
            .and_then(|w| {
                js_sys::Reflect::get(&w, &JsValue::from_str("performance"))
                    .ok()
                    .and_then(|perf| {
                        js_sys::Reflect::get(&perf, &JsValue::from_str("now"))
                            .ok()
                            .and_then(|now_fn| {
                                let now_func: js_sys::Function = now_fn.dyn_into().ok()?;
                                now_func.call0(&perf).ok()?.as_f64()
                            })
                    })
            })
            .unwrap_or_else(|| js_sys::Date::now())
    }

    /// Run game simulation step (called at fixed 60 Hz for local games)
    fn step_simulation(client: &mut Client) {
        if !client.is_local_game {
            return;
        }

        const SIM_FIXED_DT: f32 = 1.0 / 60.0; // 60 Hz fixed timestep
        let now_ms = Self::performance_now();

        // Initialize last_sim_time if needed
        if client.last_sim_time == 0.0 {
            client.last_sim_time = now_ms;
            return;
        }

        // Accumulate time for fixed timestep
        let frame_time_ms = (now_ms - client.last_sim_time) / 1000.0; // Convert to seconds
        client.sim_accumulator += frame_time_ms as f32;
        client.last_sim_time = now_ms;

        // Run simulation steps at fixed 60 Hz
        while client.sim_accumulator >= SIM_FIXED_DT {
            client.sim_accumulator -= SIM_FIXED_DT;

            if let (
                Some(ref mut world),
                Some(ref mut time),
                Some(ref map),
                Some(ref config),
                Some(ref mut score),
                Some(ref mut events),
                Some(ref mut net_queue),
                Some(ref mut rng),
                Some(ref mut respawn_state),
            ) = (
                &mut client.local_world,
                &mut client.local_time,
                &client.local_map,
                &client.local_config,
                &mut client.local_score,
                &mut client.local_events,
                &mut client.local_net_queue,
                &mut client.local_rng,
                &mut client.local_respawn_state,
            ) {
                // Add player input to queue
                net_queue.push_input(0, client.paddle_dir);

                // AI: Control right paddle (player_id=1)
                let ai_dir = Self::calculate_ai_input(world, config);
                net_queue.push_input(1, ai_dir);

                // Update time with fixed timestep
                *time = Time::new(SIM_FIXED_DT, time.now + SIM_FIXED_DT);

                // Run game simulation step
                step(
                    world,
                    time,
                    map,
                    config,
                    score,
                    events,
                    net_queue,
                    rng,
                    respawn_state,
                );

                // Extract data needed for update (before releasing borrows)
                let ball_data = world
                    .query::<&Ball>()
                    .iter()
                    .next()
                    .map(|(_e, ball)| (ball.pos, ball.vel));
                let mut paddle_left_y = 12.0;
                let mut paddle_right_y = 12.0;
                for (_e, paddle) in world.query::<&Paddle>().iter() {
                    if paddle.player_id == 0 {
                        paddle_left_y = paddle.y;
                    } else if paddle.player_id == 1 {
                        paddle_right_y = paddle.y;
                    }
                }
                let score_left = score.left;
                let score_right = score.right;
                let has_winner = score.has_winner(config.win_score);

                // Update local_score
                match &mut client.local_score {
                    Some(ref mut local_score) => {
                        local_score.left = score_left;
                        local_score.right = score_right;
                    }
                    None => {
                        client.local_score = Some(Score {
                            left: score_left,
                            right: score_right,
                        });
                    }
                }

                // Update game state for rendering
                if let Some((ball_pos, ball_vel)) = ball_data {
                    use state::GameStateSnapshot;
                    client.game_state.set_current(GameStateSnapshot {
                        ball_x: ball_pos.x,
                        ball_y: ball_pos.y,
                        paddle_left_y,
                        paddle_right_y,
                        ball_vx: ball_vel.x,
                        ball_vy: ball_vel.y,
                        tick: 0,
                    });
                }
                client.game_state.set_scores(score_left, score_right);

                // Check for win condition
                if has_winner.is_some() {
                    if let Some(ref mut local_score) = client.local_score {
                        local_score.left = 0;
                        local_score.right = 0;
                    }
                    client.game_state.set_scores(0, 0);
                }
            }
        }
    }

    #[wasm_bindgen]
    pub fn render(&mut self) -> Result<(), JsValue> {
        let client = &mut self.0;

        // Run simulation at fixed 60 Hz (separate from rendering)
        Self::step_simulation(client);

        // Run client prediction continuously at 60 Hz for online games
        if !client.is_local_game && client.predicted_world.is_some() {
            Self::step_client_prediction(client);
        }

        // Get high-precision timestamp for rendering
        let now_ms = Self::performance_now();

        // Calculate render delta time for interpolation
        let render_dt = if client.last_frame_time > 0.0 {
            ((now_ms - client.last_frame_time) / 1000.0) as f32
        } else {
            0.008 // ~120fps default
        };
        client.last_frame_time = now_ms;

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

        // Game config: 32x24 arena, paddles at x=1.5 and 30.5
        let paddle_left_x = 1.5;
        let paddle_right_x = 30.5;
        let paddle_width = 0.8;
        let paddle_height = 4.0;
        let ball_radius = 0.5;

        // Update FPS tracking (update every second, no cap for high refresh displays)
        client.fps_frame_count += 1;
        let now_sec = now_ms / 1000.0; // Convert to seconds for FPS calculation
        if now_sec - (client.fps_last_update / 1000.0) >= 1.0 {
            let time_diff_sec = (now_ms - client.fps_last_update) / 1000.0;
            let calculated_fps = client.fps_frame_count as f32 / time_diff_sec as f32;
            client.fps = calculated_fps; // No cap - support 120Hz, 144Hz, 240Hz displays
            client.fps_frame_count = 0;
            client.fps_last_update = now_ms;
        }

        // Throttle update delay display (update every 200ms)
        if now_ms - client.update_last_display >= 200.0 {
            client.update_display_ms = client.game_state.time_since_update() * 1000.0;
            client.update_last_display = now_ms;
        }

        // Update interpolation with render delta time
        client.game_state.update_interpolation(render_dt);

        // Get paddle positions: use predicted state for own paddle, server state for opponent
        let player_id = client.game_state.get_player_id().unwrap_or(0);
        let left_paddle_y =
            if !client.is_local_game && client.predicted_world.is_some() && player_id == 0 {
                // Use predicted state for own paddle (left)
                if let Some(ref world) = client.predicted_world {
                    world
                        .query::<&Paddle>()
                        .iter()
                        .find(|(_e, p)| p.player_id == 0)
                        .map(|(_e, p)| p.y)
                        .unwrap_or_else(|| client.game_state.get_paddle_left_y())
                } else {
                    client.game_state.get_paddle_left_y()
                }
            } else {
                client.game_state.get_paddle_left_y()
            };

        let right_paddle_y =
            if !client.is_local_game && client.predicted_world.is_some() && player_id == 1 {
                // Use predicted state for own paddle (right)
                if let Some(ref world) = client.predicted_world {
                    world
                        .query::<&Paddle>()
                        .iter()
                        .find(|(_e, p)| p.player_id == 1)
                        .map(|(_e, p)| p.y)
                        .unwrap_or_else(|| client.game_state.get_paddle_right_y())
                } else {
                    client.game_state.get_paddle_right_y()
                }
            } else {
                client.game_state.get_paddle_right_y()
            };

        let left_paddle_instance = InstanceData {
            transform: [paddle_left_x, left_paddle_y, paddle_width, paddle_height],
            tint: [1.0, 1.0, 1.0, 1.0], // White
        };

        let right_paddle_instance = InstanceData {
            transform: [paddle_right_x, right_paddle_y, paddle_width, paddle_height],
            tint: [1.0, 1.0, 1.0, 1.0], // White
        };

        let ball_instance = InstanceData {
            transform: [
                client.game_state.get_ball_x(),
                client.game_state.get_ball_y(),
                ball_radius * 2.0,
                ball_radius * 2.0,
            ],
            tint: [1.0, 1.0, 0.2, 1.0], // Yellowish
        };

        // Only update buffers if data changed (optimization for 120+ FPS)
        let current_instances = (left_paddle_instance, right_paddle_instance, ball_instance);
        let needs_update = client
            .last_instance_data
            .map(|last| {
                last.0.transform != current_instances.0.transform
                    || last.1.transform != current_instances.1.transform
                    || last.2.transform != current_instances.2.transform
            })
            .unwrap_or(true);

        if needs_update {
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
            client.last_instance_data = Some(current_instances);
        }

        // Trail effect using ping-pong textures (optional for performance)
        if client.enable_trails {
            // Step 1: Render current frame to the "write" trail texture
            let (_trail_read_view, trail_write_view, trail_read_bind_group) = if client.trail_use_a
            {
                (
                    &client.trail_texture_view_b,
                    &client.trail_texture_view_a,
                    &client.trail_bind_group_b,
                )
            } else {
                (
                    &client.trail_texture_view_a,
                    &client.trail_texture_view_b,
                    &client.trail_bind_group_a,
                )
            };

            // Render current frame to trail texture (write target)
            {
                let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                    label: Some("Trail Write Pass"),
                    color_attachments: &[Some(RenderPassColorAttachment {
                        view: trail_write_view,
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

                // Draw game objects to trail texture
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

                // Draw ball
                render_pass.set_vertex_buffer(0, client.circle_mesh.vertex_buffer.slice(..));
                render_pass.set_vertex_buffer(1, client.ball_instance_buffer.slice(..));
                render_pass.set_index_buffer(
                    client.circle_mesh.index_buffer.slice(..),
                    IndexFormat::Uint16,
                );
                render_pass.draw_indexed(0..client.circle_mesh.index_count, 0, 0..1);
            }

            // Step 2: Fade the previous trail texture and render to write target
            {
                let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                    label: Some("Trail Fade Pass"),
                    color_attachments: &[Some(RenderPassColorAttachment {
                        view: trail_write_view,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Load, // Load what we just rendered
                            store: StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });

                // Render faded previous frame on top (creates trail accumulation)
                render_pass.set_pipeline(&client.trail_render_pipeline);
                render_pass.set_bind_group(0, trail_read_bind_group, &[]);
                render_pass.set_vertex_buffer(0, client.trail_vertex_buffer.slice(..));
                render_pass.draw(0..4, 0..1);
            }

            // Step 3: Render to main surface (faded trail + current frame)
            {
                let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                    label: Some("Main Render Pass"),
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

                // First, draw the faded trail as background
                render_pass.set_pipeline(&client.trail_render_pipeline);
                render_pass.set_bind_group(0, trail_read_bind_group, &[]);
                render_pass.set_vertex_buffer(0, client.trail_vertex_buffer.slice(..));
                render_pass.draw(0..4, 0..1);

                // Then draw game objects on top
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

                // Draw ball (circle) - trail effect handled by shader
                render_pass.set_vertex_buffer(0, client.circle_mesh.vertex_buffer.slice(..));
                render_pass.set_vertex_buffer(1, client.ball_instance_buffer.slice(..));
                render_pass.set_index_buffer(
                    client.circle_mesh.index_buffer.slice(..),
                    IndexFormat::Uint16,
                );
                render_pass.draw_indexed(0..client.circle_mesh.index_count, 0, 0..1);
            }

            // Toggle ping-pong for next frame
            client.trail_use_a = !client.trail_use_a;
        } else {
            // No trails: Direct render to screen (much faster - 1 pass instead of 3)
            let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Main Render Pass (No Trails)"),
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

            // Draw ball
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

        // Handle ping response separately
        if let S2C::Pong { t_ms: _ } = msg {
            if let Some(sent_time) = client.ping_pending {
                let current_time = Self::performance_now(); // milliseconds since epoch
                let rtt = current_time - sent_time;
                client.ping_ms = rtt as f32;
                client.ping_pending = None;
            }
            return Ok(());
        }

        // Handle reconciliation and prediction initialization for GameState messages
        let is_game_state = matches!(msg, S2C::GameState { .. });
        let server_tick = if let S2C::GameState { tick, .. } = &msg {
            Some(*tick)
        } else {
            None
        };

        if let Some(tick) = server_tick {
            Self::reconcile_with_server(client, tick);
        }

        handle_message(msg, &mut client.game_state)
            .map_err(|e| JsValue::from_str(&format!("Failed to handle message: {}", e)))?;

        // Initialize prediction from server snapshot if not already initialized
        if is_game_state {
            if client.predicted_world.is_none() && !client.is_local_game {
                if let Some(snapshot) = client.game_state.get_current_snapshot() {
                    Self::initialize_prediction(client, &snapshot);
                }
            }
        }

        Ok(())
    }

    /// Get join message bytes
    #[wasm_bindgen]
    pub fn get_join_bytes(&self, code: String) -> Vec<u8> {
        network::create_join_message(&code).unwrap_or_default()
    }

    /// Get input message bytes (triggers client prediction)
    #[wasm_bindgen]
    pub fn get_input_bytes(&mut self) -> Vec<u8> {
        let client = &mut self.0;
        if client.is_local_game {
            // Local games don't need prediction
            let player_id = client.game_state.get_player_id().unwrap_or(0);
            return network::create_input_message(player_id, client.paddle_dir, 0)
                .unwrap_or_default();
        }

        let player_id = client.game_state.get_player_id().unwrap_or(0);
        let seq = client.input_seq;
        client.input_seq = client.input_seq.wrapping_add(1);

        // Store input in history for replay
        client.input_history.push((seq, client.paddle_dir));
        // Keep only last 120 inputs (2 seconds at 60 Hz)
        if client.input_history.len() > 120 {
            client.input_history.remove(0);
        }

        // Run client prediction immediately
        Self::run_client_prediction(client, player_id, client.paddle_dir);

        network::create_input_message(player_id, client.paddle_dir, seq).unwrap_or_default()
    }

    /// Get current score (for UI updates)
    #[wasm_bindgen]
    pub fn get_score(&self) -> Vec<u8> {
        if self.0.is_local_game {
            // For local games, always read from local_score
            // This is updated every frame in the render loop
            match &self.0.local_score {
                Some(score) => vec![score.left, score.right],
                None => vec![0, 0],
            }
        } else {
            // For online games, read from game_state
            let (left, right) = self.0.game_state.get_scores();
            vec![left, right]
        }
    }

    /// Start local game with AI opponent
    #[wasm_bindgen]
    pub fn start_local_game(&mut self) {
        let client = &mut self.0;
        client.is_local_game = true;

        // Initialize game resources
        let map = GameMap::new();
        let config = Config::new();
        let mut world = World::new();
        let mut rng = GameRng::new(Self::performance_now() as u64);

        // Create paddles
        let left_paddle_y = map.paddle_spawn(0).y;
        let right_paddle_y = map.paddle_spawn(1).y;
        create_paddle(&mut world, 0, left_paddle_y);
        create_paddle(&mut world, 1, right_paddle_y);

        // Create ball with random direction
        let mut ball = Ball::new(glam::f32::Vec2::ZERO, glam::f32::Vec2::ZERO);
        ball.reset(config.ball_speed_initial, &mut rng);
        create_ball(&mut world, ball.pos, ball.vel);

        // Initialize game state
        client.local_world = Some(world);
        client.local_time = Some(Time::new(0.016, 0.0));
        client.local_map = Some(map);
        client.local_config = Some(config);
        client.local_score = Some(Score::new());
        client.local_events = Some(Events::new());
        client.local_net_queue = Some(NetQueue::new());
        client.local_rng = Some(rng);
        client.local_respawn_state = Some(RespawnState::new());

        // Set player ID for local game
        client.game_state.set_player_id(0); // Player is left paddle
    }

    /// Initialize client prediction state from server snapshot
    fn initialize_prediction(client: &mut Client, snapshot: &state::GameStateSnapshot) {
        use game_core::create_ball;
        let map = GameMap::new();
        let config = Config::new();
        let mut world = World::new();
        let rng = GameRng::new(Self::performance_now() as u64);

        // Create paddles at server positions
        create_paddle(&mut world, 0, snapshot.paddle_left_y);
        create_paddle(&mut world, 1, snapshot.paddle_right_y);

        // Create ball at server position with server velocity
        create_ball(
            &mut world,
            glam::f32::Vec2::new(snapshot.ball_x, snapshot.ball_y),
            glam::f32::Vec2::new(snapshot.ball_vx, snapshot.ball_vy),
        );

        client.predicted_world = Some(world);
        client.predicted_time = Some(Time::new(0.016, 0.0));
        client.predicted_map = Some(map);
        client.predicted_config = Some(config);
        client.predicted_score = Some(Score::new());
        client.predicted_events = Some(Events::new());
        client.predicted_net_queue = Some(NetQueue::new());
        client.predicted_rng = Some(rng);
        client.predicted_respawn_state = Some(RespawnState::new());
        client.last_reconciled_tick = snapshot.tick;
        client.predicted_tick = snapshot.tick;
    }

    /// Run client prediction: simulate locally when input changes (one step)
    fn run_client_prediction(client: &mut Client, player_id: u8, paddle_dir: i8) {
        // Skip if prediction not initialized
        if client.predicted_world.is_none() {
            return;
        }

        const SIM_FIXED_DT: f32 = 1.0 / 60.0; // 60 Hz fixed timestep

        if let (
            Some(ref mut world),
            Some(ref mut time),
            Some(ref map),
            Some(ref config),
            Some(ref mut score),
            Some(ref mut events),
            Some(ref mut net_queue),
            Some(ref mut rng),
            Some(ref mut respawn_state),
        ) = (
            &mut client.predicted_world,
            &mut client.predicted_time,
            &client.predicted_map,
            &client.predicted_config,
            &mut client.predicted_score,
            &mut client.predicted_events,
            &mut client.predicted_net_queue,
            &mut client.predicted_rng,
            &mut client.predicted_respawn_state,
        ) {
            // Add player input
            net_queue.push_input(player_id, paddle_dir);
            // Opponent input is unknown, so we don't add it (will be corrected by server)

            // Update time
            *time = Time::new(SIM_FIXED_DT, time.now + SIM_FIXED_DT);

            // Run simulation step
            step(
                world,
                time,
                map,
                config,
                score,
                events,
                net_queue,
                rng,
                respawn_state,
            );

            // Increment predicted tick
            client.predicted_tick += 1;
        }
    }

    /// Step client prediction continuously at 60 Hz (called from render loop)
    fn step_client_prediction(client: &mut Client) {
        if client.predicted_world.is_none() {
            return;
        }

        const SIM_FIXED_DT: f32 = 1.0 / 60.0; // 60 Hz fixed timestep
        let now_ms = Self::performance_now();

        // Initialize last_prediction_time if needed
        if client.last_prediction_time == 0.0 {
            client.last_prediction_time = now_ms;
            return;
        }

        // Accumulate time for fixed timestep (same pattern as local game)
        let frame_time_ms = (now_ms - client.last_prediction_time) / 1000.0; // Convert to seconds
        client.prediction_accumulator += frame_time_ms as f32;
        client.last_prediction_time = now_ms;

        let player_id = client.game_state.get_player_id().unwrap_or(0);
        let paddle_dir = client.paddle_dir;

        // Run simulation steps at fixed 60 Hz
        while client.prediction_accumulator >= SIM_FIXED_DT {
            client.prediction_accumulator -= SIM_FIXED_DT;

            if let (
                Some(ref mut world),
                Some(ref mut time),
                Some(ref map),
                Some(ref config),
                Some(ref mut score),
                Some(ref mut events),
                Some(ref mut net_queue),
                Some(ref mut rng),
                Some(ref mut respawn_state),
            ) = (
                &mut client.predicted_world,
                &mut client.predicted_time,
                &client.predicted_map,
                &client.predicted_config,
                &mut client.predicted_score,
                &mut client.predicted_events,
                &mut client.predicted_net_queue,
                &mut client.predicted_rng,
                &mut client.predicted_respawn_state,
            ) {
                // Clear queue before adding new input (prevents accumulation)
                net_queue.clear();
                // Add current player input (continuously while key is held)
                net_queue.push_input(player_id, paddle_dir);
                // Opponent input is unknown, so we don't add it (will be corrected by server)

                // Update time
                *time = Time::new(SIM_FIXED_DT, time.now + SIM_FIXED_DT);

                // Run simulation step
                step(
                    world,
                    time,
                    map,
                    config,
                    score,
                    events,
                    net_queue,
                    rng,
                    respawn_state,
                );

                // Increment predicted tick
                client.predicted_tick += 1;
            }
        }
    }

    /// Reconcile predicted state with server state
    fn reconcile_with_server(client: &mut Client, server_tick: u32) {
        // If server tick is ahead or equal to our predicted tick, accept server state
        if server_tick >= client.predicted_tick {
            // Server is ahead or we're in sync - reset prediction state
            // This will be initialized from the server snapshot in handle_message
            client.predicted_world = None;
            client.predicted_time = None;
            client.predicted_map = None;
            client.predicted_config = None;
            client.predicted_score = None;
            client.predicted_events = None;
            client.predicted_net_queue = None;
            client.predicted_rng = None;
            client.predicted_respawn_state = None;
            client.last_reconciled_tick = server_tick;
            client.predicted_tick = server_tick;
            return;
        }

        // Server is behind - we need to rewind and replay
        // For now, we'll just reset to server state (simple approach)
        // A more sophisticated approach would rewind to server_tick and replay inputs
        client.predicted_world = None;
        client.predicted_time = None;
        client.predicted_map = None;
        client.predicted_config = None;
        client.predicted_score = None;
        client.predicted_events = None;
        client.predicted_net_queue = None;
        client.predicted_rng = None;
        client.last_reconciled_tick = server_tick;
        client.predicted_tick = server_tick;
    }

    /// Calculate AI input for opponent paddle
    /// Simple AI: move toward ball's Y position with some prediction
    fn calculate_ai_input(world: &World, config: &Config) -> i8 {
        // Find ball and right paddle
        let ball_data = world
            .query::<&Ball>()
            .iter()
            .next()
            .map(|(_e, ball)| (ball.pos, ball.vel));
        let paddle_data = world
            .query::<&Paddle>()
            .iter()
            .find(|(_e, p)| p.player_id == 1)
            .map(|(_e, p)| p.y);

        if let (Some((ball_pos, ball_vel)), Some(paddle_y)) = (ball_data, paddle_data) {
            // Only move if ball is moving toward AI (positive X velocity)
            if ball_vel.x > 0.0 {
                // Predict where ball will be when it reaches paddle
                let paddle_x = config.paddle_x(1);
                let time_to_reach = (paddle_x - ball_pos.x) / ball_vel.x.max(0.1);
                let predicted_y = ball_pos.y + ball_vel.y * time_to_reach;

                // Add some imperfection (AI isn't perfect)
                let target_y = predicted_y + (ball_vel.y * 0.3); // Slight over/under correction

                // Move toward target with deadzone
                let diff = target_y - paddle_y;
                let deadzone = 0.3; // Don't move if very close

                if diff > deadzone {
                    1 // Move down
                } else if diff < -deadzone {
                    -1 // Move up
                } else {
                    0 // Stop
                }
            } else {
                // Ball moving away, move toward center
                let center_y = 12.0; // Arena center
                let diff = center_y - paddle_y;
                if diff.abs() > 0.5 {
                    if diff > 0.0 {
                        1
                    } else {
                        -1
                    }
                } else {
                    0
                }
            }
        } else {
            0 // No ball or paddle found
        }
    }

    /// Get performance metrics: [fps, ping_ms, state_delay_ms]
    /// state_delay_ms: Time since last game state update from server (in milliseconds, throttled)
    /// In local mode, ping and update are 0
    #[wasm_bindgen]
    pub fn get_metrics(&self) -> Vec<f32> {
        if self.0.is_local_game {
            vec![
                self.0.fps, 0.0, // No ping in local mode
                0.0, // No update delay in local mode
            ]
        } else {
            vec![
                self.0.fps,
                self.0.ping_ms,
                self.0.update_display_ms, // Throttled display value
            ]
        }
    }

    /// Send ping to server for latency measurement
    #[wasm_bindgen]
    pub fn send_ping(&mut self) -> Vec<u8> {
        let client = &mut self.0;
        let now_ms = Self::performance_now(); // milliseconds since epoch
        let t_ms = now_ms as u32; // Send as u32 for protocol compatibility
        client.ping_pending = Some(now_ms); // Store full precision for calculation
        network::create_ping_message(t_ms).unwrap_or_default()
    }

    /// Handle key down event
    #[wasm_bindgen]
    pub fn on_key_down(&mut self, event: KeyboardEvent) {
        let key = input::get_key_from_event(&event);
        self.0.paddle_dir = input::handle_key_down(&key, self.0.paddle_dir);
    }

    /// Handle key up event
    #[wasm_bindgen]
    pub fn on_key_up(&mut self, event: KeyboardEvent) {
        let key = input::get_key_from_event(&event);
        self.0.paddle_dir = input::handle_key_up(&key, self.0.paddle_dir);
    }

    /// Handle key by string (for touch controls)
    #[wasm_bindgen]
    pub fn handle_key_string(&mut self, key: String, is_down: bool) {
        if is_down {
            self.0.paddle_dir = input::handle_key_down(&key, self.0.paddle_dir);
        } else {
            self.0.paddle_dir = input::handle_key_up(&key, self.0.paddle_dir);
        }
    }
}
