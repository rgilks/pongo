//! Mesh generation for 2D shapes

use wgpu::util::DeviceExt;
use wgpu::*;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
}

pub struct Mesh {
    pub vertex_buffer: Buffer,
    pub index_buffer: Buffer,
    pub index_count: u32,
}

pub fn create_rectangle(device: &Device) -> Mesh {
    // Rectangle vertices: centered at origin, 1x1 unit
    let vertices = [
        Vertex {
            position: [-0.5, -0.5, 0.0],
        }, // Bottom-left
        Vertex {
            position: [0.5, -0.5, 0.0],
        }, // Bottom-right
        Vertex {
            position: [0.5, 0.5, 0.0],
        }, // Top-right
        Vertex {
            position: [-0.5, 0.5, 0.0],
        }, // Top-left
    ];

    let indices: [u16; 6] = [0, 1, 2, 0, 2, 3]; // Two triangles

    let vertex_buffer = device.create_buffer_init(&util::BufferInitDescriptor {
        label: Some("Rectangle Vertex Buffer"),
        contents: bytemuck::cast_slice(&vertices),
        usage: BufferUsages::VERTEX,
    });

    let index_buffer = device.create_buffer_init(&util::BufferInitDescriptor {
        label: Some("Rectangle Index Buffer"),
        contents: bytemuck::cast_slice(&indices),
        usage: BufferUsages::INDEX,
    });

    Mesh {
        vertex_buffer,
        index_buffer,
        index_count: 6,
    }
}

pub fn create_circle(device: &Device, segments: u32) -> Mesh {
    let mut vertices = Vec::with_capacity((segments + 1) as usize);
    let mut indices = Vec::with_capacity((segments * 3) as usize);

    // Center vertex
    vertices.push(Vertex {
        position: [0.0, 0.0, 0.0],
    });

    // Circle vertices
    for i in 0..=segments {
        let angle = 2.0 * std::f32::consts::PI * (i as f32) / (segments as f32);
        vertices.push(Vertex {
            position: [0.5 * angle.cos(), 0.5 * angle.sin(), 0.0],
        });
    }

    // Triangle indices (fan from center)
    for i in 0..segments {
        indices.push(0); // Center
        indices.push((i + 1) as u16);
        indices.push((i + 2) as u16);
    }

    let vertex_buffer = device.create_buffer_init(&util::BufferInitDescriptor {
        label: Some("Circle Vertex Buffer"),
        contents: bytemuck::cast_slice(&vertices),
        usage: BufferUsages::VERTEX,
    });

    let index_buffer = device.create_buffer_init(&util::BufferInitDescriptor {
        label: Some("Circle Index Buffer"),
        contents: bytemuck::cast_slice(&indices),
        usage: BufferUsages::INDEX,
    });

    Mesh {
        vertex_buffer,
        index_buffer,
        index_count: (segments * 3) as u32,
    }
}
