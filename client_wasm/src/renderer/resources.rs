use crate::camera::{Camera, CameraUniform};
use wgpu::util::DeviceExt;
use wgpu::*;

/// Instance data for rendering (matches shader InstanceInput).
/// Must use `repr(C)` and `bytemuck` to safely cast to raw bytes for the GPU buffer.
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct InstanceData {
    pub transform: [f32; 4], // x, y, scale_x, scale_y
    pub tint: [f32; 4],      // rgba
}

pub struct GameBuffers {
    pub camera: Buffer,
    pub left_paddle: Buffer,
    pub right_paddle: Buffer,
    pub ball: Buffer,
    pub trail_vertex: Buffer,
}

#[allow(dead_code)]
pub struct TrailTextures {
    pub texture_a: Texture,
    pub texture_b: Texture,
    pub view_a: TextureView,
    pub view_b: TextureView,
    pub sampler: Sampler,
}

pub fn create_buffers(device: &Device, camera: &Camera) -> GameBuffers {
    // Camera buffer
    let mut camera_uniform = CameraUniform::new();
    camera_uniform.update_view_proj(camera);

    let camera_buffer = device.create_buffer_init(&util::BufferInitDescriptor {
        label: Some("Camera Buffer"),
        contents: bytemuck::cast_slice(&[camera_uniform]),
        usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
    });

    // Instance buffers
    let instance_buffer_size = std::mem::size_of::<InstanceData>() as u64;

    let left_paddle = device.create_buffer(&BufferDescriptor {
        label: Some("Left Paddle Instance Buffer"),
        size: instance_buffer_size,
        usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let right_paddle = device.create_buffer(&BufferDescriptor {
        label: Some("Right Paddle Instance Buffer"),
        size: instance_buffer_size,
        usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let ball = device.create_buffer(&BufferDescriptor {
        label: Some("Ball Instance Buffer"),
        size: instance_buffer_size,
        usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    // Trail quad
    let trail_vertices: [f32; 16] = [
        -1.0, -1.0, 0.0, 1.0, 1.0, -1.0, 1.0, 1.0, -1.0, 1.0, 0.0, 0.0, 1.0, 1.0, 1.0, 0.0,
    ];
    let trail_vertex = device.create_buffer_init(&util::BufferInitDescriptor {
        label: Some("Trail Vertex Buffer"),
        contents: bytemuck::cast_slice(&trail_vertices),
        usage: BufferUsages::VERTEX,
    });

    GameBuffers {
        camera: camera_buffer,
        left_paddle,
        right_paddle,
        ball,
        trail_vertex,
    }
}

pub fn create_trail_textures(device: &Device, config: &SurfaceConfiguration) -> TrailTextures {
    let texture_desc = TextureDescriptor {
        label: Some("Trail Texture"),
        size: Extent3d {
            width: config.width,
            height: config.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: config.format,
        usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    };

    let texture_a = device.create_texture(&TextureDescriptor {
        label: Some("Trail Texture A"),
        ..texture_desc
    });

    let texture_b = device.create_texture(&TextureDescriptor {
        label: Some("Trail Texture B"),
        ..texture_desc
    });

    let view_a = texture_a.create_view(&TextureViewDescriptor::default());
    let view_b = texture_b.create_view(&TextureViewDescriptor::default());

    let sampler = device.create_sampler(&SamplerDescriptor {
        label: Some("Trail Sampler"),
        address_mode_u: AddressMode::ClampToEdge,
        address_mode_v: AddressMode::ClampToEdge,
        address_mode_w: AddressMode::ClampToEdge,
        mag_filter: FilterMode::Linear,
        min_filter: FilterMode::Linear,
        mipmap_filter: FilterMode::Linear,
        ..Default::default()
    });

    TrailTextures {
        texture_a,
        texture_b,
        view_a,
        view_b,
        sampler,
    }
}
