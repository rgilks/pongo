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

        // Create vertex buffer layout
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

        // Create render pipeline
        let render_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[vertex_buffer_layout],
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

            // Draw ground quad
            render_pass.set_vertex_buffer(0, self.ground_mesh.vertex_buffer.slice(..));
            render_pass
                .set_index_buffer(self.ground_mesh.index_buffer.slice(..), IndexFormat::Uint16);
            render_pass.draw_indexed(0..self.ground_mesh.index_count, 0, 0..1);
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
