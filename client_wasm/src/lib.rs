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

/// Previous and current game state for interpolation
#[derive(Clone)]
struct GameStateSnapshot {
    ball_x: f32,
    ball_y: f32,
    paddle_left_y: f32,
    paddle_right_y: f32,
    ball_vx: f32,
    ball_vy: f32,
    tick: u32,
}

/// Game state tracking with interpolation
struct GameState {
    // Current authoritative state from server
    current: GameStateSnapshot,
    // Previous state for interpolation
    previous: GameStateSnapshot,
    // Interpolation time (0.0 = previous, 1.0 = current)
    interpolation_alpha: f32,
    // Time since last state update
    time_since_update: f32,
    // Score (doesn't need interpolation)
    score_left: u8,
    score_right: u8,
    my_player_id: Option<u8>,
}

impl GameState {
    fn new() -> Self {
        let initial = GameStateSnapshot {
            ball_x: 16.0,
            ball_y: 12.0,
            paddle_left_y: 12.0,
            paddle_right_y: 12.0,
            ball_vx: 0.0,
            ball_vy: 0.0,
            tick: 0,
        };
        Self {
            current: initial.clone(),
            previous: initial,
            interpolation_alpha: 1.0,
            time_since_update: 0.0,
            score_left: 0,
            score_right: 0,
            my_player_id: None,
        }
    }

    /// Update interpolation based on elapsed time
    /// Target: 60fps render, 20-60Hz server updates
    fn update_interpolation(&mut self, dt: f32) {
        self.time_since_update += dt;
        // Interpolate over ~60ms (20Hz update rate = 1/20 = 0.05s)
        // Use slightly longer duration to handle jitter and ensure smoothness
        let interpolation_duration = 0.06; // 60ms for smoother interpolation
        self.interpolation_alpha = (self.time_since_update / interpolation_duration).min(1.0);
    }

    /// Get interpolated position
    fn interpolate(&self, prev: f32, curr: f32) -> f32 {
        prev + (curr - prev) * self.interpolation_alpha
    }

    /// Get current interpolated positions
    fn get_ball_x(&self) -> f32 {
        self.interpolate(self.previous.ball_x, self.current.ball_x)
    }

    fn get_ball_y(&self) -> f32 {
        self.interpolate(self.previous.ball_y, self.current.ball_y)
    }

    fn get_paddle_left_y(&self) -> f32 {
        self.interpolate(self.previous.paddle_left_y, self.current.paddle_left_y)
    }

    fn get_paddle_right_y(&self) -> f32 {
        self.interpolate(self.previous.paddle_right_y, self.current.paddle_right_y)
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
    // Frame timing for interpolation
    last_frame_time: f64,
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

        // Calculate frame delta time for interpolation
        let now = js_sys::Date::now() / 1000.0; // Convert to seconds
        let dt = if client.last_frame_time > 0.0 {
            (now - client.last_frame_time) as f32
        } else {
            0.016 // ~60fps default
        };
        client.last_frame_time = now;

        // Update interpolation
        client.game_state.update_interpolation(dt);

        let left_paddle_instance = InstanceData {
            transform: [
                paddle_left_x,
                client.game_state.get_paddle_left_y(),
                paddle_width,
                paddle_height,
            ],
            tint: [1.0, 1.0, 1.0, 1.0], // White
        };

        let right_paddle_instance = InstanceData {
            transform: [
                paddle_right_x,
                client.game_state.get_paddle_right_y(),
                paddle_width,
                paddle_height,
            ],
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

        // Trail effect using ping-pong textures
        // Step 1: Render current frame to the "write" trail texture
        let (trail_read_view, trail_write_view, trail_read_bind_group) = if client.trail_use_a {
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
            }
            S2C::GameState {
                ball_x,
                ball_y,
                paddle_left_y,
                paddle_right_y,
                score_left,
                score_right,
                ball_vx,
                ball_vy,
                tick,
            } => {
                // Store previous state for interpolation
                client.game_state.previous = client.game_state.current.clone();

                // Update current state
                client.game_state.current = GameStateSnapshot {
                    ball_x,
                    ball_y,
                    paddle_left_y,
                    paddle_right_y,
                    ball_vx,
                    ball_vy,
                    tick,
                };

                // Reset interpolation timer
                client.game_state.time_since_update = 0.0;
                client.game_state.interpolation_alpha = 0.0;

                // Update scores
                client.game_state.score_left = score_left;
                client.game_state.score_right = score_right;
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

    /// Get current score (for UI updates)
    #[wasm_bindgen]
    pub fn get_score(&self) -> Vec<u8> {
        vec![self.0.game_state.score_left, self.0.game_state.score_right]
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
