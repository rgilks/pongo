pub mod init;
pub mod pipeline;
pub mod resources;
pub mod shaders;
pub mod draw; // Add draw module

use crate::camera::Camera;
use crate::mesh::{create_circle, create_rectangle, Mesh};
use crate::state::GameState;
use resources::{GameBuffers, TrailTextures, InstanceData};
use wgpu::*;

#[allow(dead_code)]
pub struct Renderer {
    pub device: Device,
    pub queue: Queue,
    pub surface: Surface<'static>,
    pub surface_config: SurfaceConfiguration,
    pub size: (u32, u32),
    pub camera: Camera,
    
    // Pipelines
    pub main_pipeline: RenderPipeline,
    pub trail_pipeline: RenderPipeline,
    
    // Bind Groups
    pub camera_bind_group: BindGroup,
    pub trail_bind_group_a: BindGroup,
    pub trail_bind_group_b: BindGroup,

    // Resources
    pub buffers: GameBuffers,
    pub textures: TrailTextures,
    pub meshes: (Mesh, Mesh), // rect, circle

    // State
    pub trail_use_a: bool,
    pub last_instance_data: Option<(InstanceData, InstanceData, InstanceData)>,
    pub enable_trails: bool,
}

impl Renderer {
    pub async fn new(canvas: web_sys::HtmlCanvasElement) -> Result<Self, String> {
        let ctx = init::init_wgpu(canvas).await?;
        let camera = Camera::orthographic(32.0, 24.0);

        let buffers = resources::create_buffers(&ctx.device, &camera);
        let textures = resources::create_trail_textures(&ctx.device, &ctx.config);
        let pipes = pipeline::create_pipelines(&ctx.device, ctx.config.format);

        // Meshes
        let rect_mesh = create_rectangle(&ctx.device);
        let circle_mesh = create_circle(&ctx.device, 32);

        // Bind Groups
        let camera_bind_group = ctx.device.create_bind_group(&BindGroupDescriptor {
            label: Some("Camera Bind Group"),
            layout: &pipes.camera_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: buffers.camera.as_entire_binding(),
            }],
        });

        let trail_bind_group_a = ctx.device.create_bind_group(&BindGroupDescriptor {
            label: Some("Trail Bind Group A"),
            layout: &pipes.trail_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&textures.view_a),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&textures.sampler),
                },
            ],
        });

        let trail_bind_group_b = ctx.device.create_bind_group(&BindGroupDescriptor {
            label: Some("Trail Bind Group B"),
            layout: &pipes.trail_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&textures.view_b),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&textures.sampler),
                },
            ],
        });

        Ok(Self {
            device: ctx.device,
            queue: ctx.queue,
            surface: ctx.surface,
            surface_config: ctx.config,
            size: ctx.size,
            camera,
            main_pipeline: pipes.main_pipeline,
            trail_pipeline: pipes.trail_pipeline,
            camera_bind_group,
            trail_bind_group_a,
            trail_bind_group_b,
            buffers,
            textures,
            meshes: (rect_mesh, circle_mesh),
            trail_use_a: true,
            last_instance_data: None,
            enable_trails: true,
        })
    }

    pub fn draw(
        &mut self,
        game_state: &GameState,
        local_paddle_y: f32,
        is_local_game: bool,
    ) -> Result<(), String> {
        draw::draw_frame(self, game_state, local_paddle_y, is_local_game)
    }
}
