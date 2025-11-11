//! 2D orthographic camera

use glam::Mat4;

pub struct Camera {
    pub view: Mat4,
    pub projection: Mat4,
}

impl Camera {
    pub fn orthographic(width: f32, height: f32) -> Self {
        let view = Mat4::IDENTITY;
        // Orthographic projection: left=0, right=width, bottom=0, top=height
        let projection = Mat4::orthographic_rh(0.0, width, 0.0, height, -1.0, 1.0);

        Self { view, projection }
    }

    pub fn view_proj(&self) -> Mat4 {
        self.projection * self.view
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    pub view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    pub fn new() -> Self {
        Self {
            view_proj: Mat4::IDENTITY.to_cols_array_2d(),
        }
    }

    pub fn update_view_proj(&mut self, camera: &Camera) {
        self.view_proj = camera.view_proj().to_cols_array_2d();
    }
}
