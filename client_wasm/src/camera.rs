//! Isometric camera for ISO game
//!
//! Camera with pitch ~35° and user yaw offset for map rotation

use glam::{Mat4, Vec3};

/// Isometric camera parameters
pub struct Camera {
    /// Camera position in 3D space
    pub eye: Vec3,
    /// Target point to look at (center of arena)
    pub target: Vec3,
    /// Up vector
    pub up: Vec3,
    /// Pitch angle in radians (~35° = 0.6109 rad)
    #[allow(dead_code)] // Used in set_yaw_offset
    pub pitch: f32,
    /// User yaw offset for map rotation (in radians)
    #[allow(dead_code)] // Will be used for map rotation feature
    pub yaw_offset: f32,
    /// Distance from target
    #[allow(dead_code)] // Used in set_yaw_offset
    pub distance: f32,
    /// Field of view (vertical) in radians
    pub fov: f32,
    /// Aspect ratio (width / height)
    pub aspect: f32,
    /// Near clipping plane
    pub near: f32,
    /// Far clipping plane
    pub far: f32,
}

impl Camera {
    /// Create a new isometric camera
    pub fn new(aspect: f32) -> Self {
        // Isometric camera: pitch ~35° (0.6109 radians)
        let pitch = 35.0_f32.to_radians();
        let distance = 20.0; // Distance from arena center
        let yaw_offset = 0.0;

        // Calculate eye position based on pitch and distance
        // For isometric view, we want to look down at an angle
        let yaw_offset_f32: f32 = yaw_offset;
        let eye = Vec3::new(
            distance * pitch.cos() * yaw_offset_f32.cos(),
            distance * pitch.sin(),
            distance * pitch.cos() * yaw_offset_f32.sin(),
        );

        Self {
            eye,
            target: Vec3::ZERO, // Look at arena center
            up: Vec3::Y,        // Y is up
            pitch,
            yaw_offset,
            distance,
            fov: 60.0_f32.to_radians(), // 60° FOV
            aspect,
            near: 0.1,
            far: 100.0,
        }
    }

    /// Update aspect ratio (call on window resize)
    pub fn set_aspect(&mut self, aspect: f32) {
        self.aspect = aspect;
    }

    /// Set user yaw offset for map rotation
    #[allow(dead_code)] // Will be used for map rotation feature
    pub fn set_yaw_offset(&mut self, yaw_offset: f32) {
        self.yaw_offset = yaw_offset;
        // Recalculate eye position
        self.eye = Vec3::new(
            self.distance * self.pitch.cos() * yaw_offset.cos(),
            self.distance * self.pitch.sin(),
            self.distance * self.pitch.cos() * yaw_offset.sin(),
        );
    }

    /// Get view matrix (world to camera space)
    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(self.eye, self.target, self.up)
    }

    /// Get projection matrix (camera to clip space)
    pub fn projection_matrix(&self) -> Mat4 {
        Mat4::perspective_rh(self.fov, self.aspect, self.near, self.far)
    }

    /// Get view-projection matrix (combined)
    #[allow(dead_code)] // Will be used when rendering pipeline is implemented
    pub fn view_projection_matrix(&self) -> Mat4 {
        self.projection_matrix() * self.view_matrix()
    }
}

/// Camera uniform buffer data (256-byte aligned for UBO)
#[repr(C, align(256))]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    /// View-projection matrix (64 bytes)
    pub view_proj: [[f32; 4]; 4],
    /// View matrix (64 bytes)
    pub view: [[f32; 4]; 4],
    /// Projection matrix (64 bytes)
    pub proj: [[f32; 4]; 4],
    /// Camera position (12 bytes + 4 padding)
    pub eye: [f32; 4],
    /// Padding to 256 bytes
    pub _padding: [f32; 12],
}

impl CameraUniform {
    /// Create camera uniform from camera
    pub fn from_camera(camera: &Camera) -> Self {
        let view = camera.view_matrix();
        let proj = camera.projection_matrix();
        let view_proj = proj * view;

        Self {
            view_proj: view_proj.to_cols_array_2d(),
            view: view.to_cols_array_2d(),
            proj: proj.to_cols_array_2d(),
            eye: [camera.eye.x, camera.eye.y, camera.eye.z, 1.0],
            _padding: [0.0; 12],
        }
    }
}
