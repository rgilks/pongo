use wgpu::*;
use crate::mesh::Vertex;
use super::resources::InstanceData;
use super::shaders::{PONG_SHADER, TRAIL_SHADER};

pub struct PipelineState {
    pub main_pipeline: RenderPipeline,
    pub trail_pipeline: RenderPipeline,
    pub camera_layout: BindGroupLayout,
    pub trail_layout: BindGroupLayout,
}

pub fn create_pipelines(device: &Device, format: TextureFormat) -> PipelineState {
    // 1. Camera Bind Group Layout
    let camera_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
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

    // 2. Trail Bind Group Layout
    let trail_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
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

    // 3. Main Pipeline
    let shader = device.create_shader_module(ShaderModuleDescriptor {
        label: Some("Pong Shader"),
        source: ShaderSource::Wgsl(PONG_SHADER.into()),
    });

    let main_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: Some("Render Pipeline Layout"),
        bind_group_layouts: &[&camera_layout],
        push_constant_ranges: &[],
    });

    let vertex_buffer_layout = VertexBufferLayout {
        array_stride: std::mem::size_of::<Vertex>() as u64,
        step_mode: VertexStepMode::Vertex,
        attributes: &[VertexAttribute {
            offset: 0,
            shader_location: 0,
            format: VertexFormat::Float32x3,
        }],
    };

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

    let main_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
        label: Some("Render Pipeline"),
        layout: Some(&main_layout),
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
                format,
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

    // 4. Trail Pipeline
    let trail_shader = device.create_shader_module(ShaderModuleDescriptor {
        label: Some("Trail Shader"),
        source: ShaderSource::Wgsl(TRAIL_SHADER.into()),
    });

    let trail_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: Some("Trail Pipeline Layout"),
        bind_group_layouts: &[&trail_layout],
        push_constant_ranges: &[],
    });

    let trail_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
        label: Some("Trail Render Pipeline"),
        layout: Some(&trail_pipeline_layout),
        vertex: VertexState {
            module: &trail_shader,
            entry_point: Some("vs_main"),
            buffers: &[VertexBufferLayout {
                array_stride: 16,
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
                format,
                blend: Some(BlendState::ALPHA_BLENDING),
                write_mask: ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: PrimitiveState {
            topology: PrimitiveTopology::TriangleStrip,
            // ...
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

    PipelineState {
        main_pipeline,
        trail_pipeline,
        camera_layout,
        trail_layout,
    }
}
