//! Camera for Pong game
//!
//! Simple 2D orthographic camera

use glam::Mat4;

/// Camera struct
pub struct Camera {
    pub view: Mat4,
    pub projection: Mat4,
}

impl Camera {
    /// Create an orthographic camera for 2D game
    /// Arena is `width` x `height` units
    /// Coordinates: (0, 0) is bottom-left, (width, height) is top-right
    pub fn orthographic(width: f32, height: f32) -> Self {
        // For 2D, we don't need a view matrix - just use identity
        // The projection matrix handles the coordinate transformation
        let view = Mat4::IDENTITY;

        // Orthographic projection: maps world space (0,0) to (width, height)
        // to clip space (-1,-1) to (1, 1)
        // Note: orthographic_rh uses (left, right, bottom, top, near, far)
        // We want Y=0 at bottom, Y=height at top
        let projection = Mat4::orthographic_rh(0.0, width, 0.0, height, -1.0, 1.0);

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
