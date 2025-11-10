//! Mesh generation for Pong game
//!
//! Simple 2D meshes: rectangle (paddle), circle (ball)

use wgpu::util::DeviceExt;
use wgpu::*;

/// Vertex data for meshes (just position for 2D)
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3], // x, y, z (z always 0 for 2D)
}

/// Mesh data with GPU buffers
pub struct Mesh {
    pub vertex_buffer: Buffer,
    pub index_buffer: Buffer,
    pub index_count: u32,
}

impl Mesh {
    pub fn new(device: &Device, vertices: &[Vertex], indices: &[u16]) -> Self {
        let vertex_buffer = device.create_buffer_init(&util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(vertices),
            usage: BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(indices),
            usage: BufferUsages::INDEX,
        });

        Self {
            vertex_buffer,
            index_buffer,
            index_count: indices.len() as u32,
        }
    }
}

/// Create a unit rectangle (1x1) centered at origin
/// Will be scaled by instance data
pub fn create_rectangle(device: &Device) -> Mesh {
    let vertices = vec![
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

    let indices = vec![0, 1, 2, 2, 3, 0]; // Two triangles

    Mesh::new(device, &vertices, &indices)
}

/// Create a unit circle (radius 1) centered at origin
/// Will be scaled by instance data
pub fn create_circle(device: &Device, segments: u32) -> Mesh {
    let mut vertices = vec![Vertex {
        position: [0.0, 0.0, 0.0],
    }]; // Center vertex

    // Generate vertices around the circle
    for i in 0..=segments {
        let angle = 2.0 * std::f32::consts::PI * i as f32 / segments as f32;
        let x = angle.cos();
        let y = angle.sin();
        vertices.push(Vertex {
            position: [x, y, 0.0],
        });
    }

    // Generate indices (triangle fan from center)
    let mut indices = Vec::new();
    for i in 1..=segments {
        indices.push(0); // Center
        indices.push(i as u16);
        indices.push((i % segments + 1) as u16);
    }

    Mesh::new(device, &vertices, &indices)
}
