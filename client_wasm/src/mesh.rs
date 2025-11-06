//! Mesh generation for ISO game
//!
//! Simple meshes: unit sphere (eye/bolt), unit cube (block), ground quad

use wgpu::*;

/// Vertex data for meshes
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
}

/// Generate vertices and indices for a unit sphere
pub fn create_sphere(segments: u32) -> (Vec<Vertex>, Vec<u16>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    // Generate sphere vertices
    for i in 0..=segments {
        let theta = std::f32::consts::PI * i as f32 / segments as f32;
        let sin_theta = theta.sin();
        let cos_theta = theta.cos();

        for j in 0..=segments {
            let phi = 2.0 * std::f32::consts::PI * j as f32 / segments as f32;
            let sin_phi = phi.sin();
            let cos_phi = phi.cos();

            let x = sin_theta * cos_phi;
            let y = cos_theta;
            let z = sin_theta * sin_phi;

            vertices.push(Vertex {
                position: [x, y, z],
                normal: [x, y, z], // Normal is same as position for unit sphere
            });
        }
    }

    // Generate indices
    for i in 0..segments {
        for j in 0..segments {
            let first = (i * (segments + 1) + j) as u16;
            let second = first + (segments + 1) as u16;

            indices.push(first);
            indices.push(second);
            indices.push(first + 1);

            indices.push(second);
            indices.push(second + 1);
            indices.push(first + 1);
        }
    }

    (vertices, indices)
}

/// Generate vertices and indices for a unit cube
pub fn create_cube() -> (Vec<Vertex>, Vec<u16>) {
    let vertices = vec![
        // Front face
        Vertex {
            position: [-0.5, -0.5, 0.5],
            normal: [0.0, 0.0, 1.0],
        },
        Vertex {
            position: [0.5, -0.5, 0.5],
            normal: [0.0, 0.0, 1.0],
        },
        Vertex {
            position: [0.5, 0.5, 0.5],
            normal: [0.0, 0.0, 1.0],
        },
        Vertex {
            position: [-0.5, 0.5, 0.5],
            normal: [0.0, 0.0, 1.0],
        },
        // Back face
        Vertex {
            position: [-0.5, -0.5, -0.5],
            normal: [0.0, 0.0, -1.0],
        },
        Vertex {
            position: [-0.5, 0.5, -0.5],
            normal: [0.0, 0.0, -1.0],
        },
        Vertex {
            position: [0.5, 0.5, -0.5],
            normal: [0.0, 0.0, -1.0],
        },
        Vertex {
            position: [0.5, -0.5, -0.5],
            normal: [0.0, 0.0, -1.0],
        },
        // Top face
        Vertex {
            position: [-0.5, 0.5, -0.5],
            normal: [0.0, 1.0, 0.0],
        },
        Vertex {
            position: [-0.5, 0.5, 0.5],
            normal: [0.0, 1.0, 0.0],
        },
        Vertex {
            position: [0.5, 0.5, 0.5],
            normal: [0.0, 1.0, 0.0],
        },
        Vertex {
            position: [0.5, 0.5, -0.5],
            normal: [0.0, 1.0, 0.0],
        },
        // Bottom face
        Vertex {
            position: [-0.5, -0.5, -0.5],
            normal: [0.0, -1.0, 0.0],
        },
        Vertex {
            position: [0.5, -0.5, -0.5],
            normal: [0.0, -1.0, 0.0],
        },
        Vertex {
            position: [0.5, -0.5, 0.5],
            normal: [0.0, -1.0, 0.0],
        },
        Vertex {
            position: [-0.5, -0.5, 0.5],
            normal: [0.0, -1.0, 0.0],
        },
        // Right face
        Vertex {
            position: [0.5, -0.5, -0.5],
            normal: [1.0, 0.0, 0.0],
        },
        Vertex {
            position: [0.5, 0.5, -0.5],
            normal: [1.0, 0.0, 0.0],
        },
        Vertex {
            position: [0.5, 0.5, 0.5],
            normal: [1.0, 0.0, 0.0],
        },
        Vertex {
            position: [0.5, -0.5, 0.5],
            normal: [1.0, 0.0, 0.0],
        },
        // Left face
        Vertex {
            position: [-0.5, -0.5, -0.5],
            normal: [-1.0, 0.0, 0.0],
        },
        Vertex {
            position: [-0.5, -0.5, 0.5],
            normal: [-1.0, 0.0, 0.0],
        },
        Vertex {
            position: [-0.5, 0.5, 0.5],
            normal: [-1.0, 0.0, 0.0],
        },
        Vertex {
            position: [-0.5, 0.5, -0.5],
            normal: [-1.0, 0.0, 0.0],
        },
    ];

    let indices = vec![
        0, 1, 2, 2, 3, 0, // front
        4, 5, 6, 6, 7, 4, // back
        8, 9, 10, 10, 11, 8, // top
        12, 13, 14, 14, 15, 12, // bottom
        16, 17, 18, 18, 19, 16, // right
        20, 21, 22, 22, 23, 20, // left
    ];

    (vertices, indices)
}

/// Generate vertices and indices for a ground quad
pub fn create_ground_quad(size: f32) -> (Vec<Vertex>, Vec<u16>) {
    let half_size = size * 0.5;
    let vertices = vec![
        Vertex {
            position: [-half_size, 0.0, -half_size],
            normal: [0.0, 1.0, 0.0],
        },
        Vertex {
            position: [half_size, 0.0, -half_size],
            normal: [0.0, 1.0, 0.0],
        },
        Vertex {
            position: [half_size, 0.0, half_size],
            normal: [0.0, 1.0, 0.0],
        },
        Vertex {
            position: [-half_size, 0.0, half_size],
            normal: [0.0, 1.0, 0.0],
        },
    ];

    let indices = vec![0, 1, 2, 2, 3, 0];

    (vertices, indices)
}

/// Mesh data with GPU buffers
pub struct Mesh {
    pub vertex_buffer: Buffer,
    pub index_buffer: Buffer,
    pub index_count: u32,
}

impl Mesh {
    pub fn new(device: &Device, queue: &Queue, vertices: &[Vertex], indices: &[u16]) -> Self {
        let vertex_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Vertex Buffer"),
            size: std::mem::size_of_val(vertices) as u64,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&vertex_buffer, 0, bytemuck::cast_slice(vertices));

        let index_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Index Buffer"),
            size: std::mem::size_of_val(indices) as u64,
            usage: BufferUsages::INDEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&index_buffer, 0, bytemuck::cast_slice(indices));

        Self {
            vertex_buffer,
            index_buffer,
            index_count: indices.len() as u32,
        }
    }
}
