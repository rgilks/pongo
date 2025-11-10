//! Camera for Pong game
//!
//! Simple 2D orthographic camera

use glam::{Mat4, Vec3};

/// Camera struct
pub struct Camera {
    pub view: Mat4,
    pub projection: Mat4,
}

impl Camera {
    /// Create an orthographic camera for 2D game
    /// Arena is `width` x `height` units
    pub fn orthographic(width: f32, height: f32) -> Self {
        // Position camera looking down at the arena
        let eye = Vec3::new(width / 2.0, height / 2.0, 10.0);
        let target = Vec3::new(width / 2.0, height / 2.0, 0.0);
        let up = Vec3::Y;
        let view = Mat4::look_at_rh(eye, target, up);

        // Orthographic projection (0, 0) to (width, height)
        let projection = Mat4::orthographic_rh(0.0, width, 0.0, height, 0.1, 100.0);

        Self { view, projection }
    }
}

/// Camera uniform data (matches WGSL struct, 256-byte aligned)
#[repr(C, align(256))]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    view_proj: [[f32; 4]; 4], // 64 bytes (mat4x4)
    _padding: [f32; 48],      // 192 bytes padding (48 * 4) to reach 256 bytes
}

impl CameraUniform {
    pub fn from_camera(camera: &Camera) -> Self {
        let view_proj = camera.projection * camera.view;
        Self {
            view_proj: view_proj.to_cols_array_2d(),
            _padding: [0.0; 48],
        }
    }
}
