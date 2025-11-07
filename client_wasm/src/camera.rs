//! First-person camera for ISO game
//!
//! Camera positioned at player's eye level, looking in the direction they're facing

use glam::{Mat4, Vec3};

/// First-person camera parameters
pub struct Camera {
    /// Camera position in 3D space (player eye position)
    pub eye: Vec3,
    /// Target point to look at (forward direction from player)
    pub target: Vec3,
    /// Up vector
    pub up: Vec3,
    /// Field of view (vertical) in radians
    pub fov: f32,
    /// Aspect ratio (width / height)
    pub aspect: f32,
    /// Near clipping plane
    pub near: f32,
    /// Far clipping plane
    pub far: f32,
    /// Eye height above ground (for first-person view)
    pub eye_height: f32,
}

impl Camera {
    /// Create a new first-person camera
    pub fn new(aspect: f32) -> Self {
        Self {
            eye: Vec3::new(0.0, 1.0, 0.0),    // Start at origin, eye level
            target: Vec3::new(1.0, 1.0, 0.0), // Look forward (+X direction)
            up: Vec3::Y,                      // Y is up
            fov: 75.0_f32.to_radians(),       // 75° FOV for first-person
            aspect,
            near: 0.1,
            far: 100.0,
            eye_height: 1.0, // Eye level 1 unit above ground
        }
    }

    /// Update aspect ratio (call on window resize)
    pub fn set_aspect(&mut self, aspect: f32) {
        self.aspect = aspect;
    }

    /// Update camera to first-person view from player position and yaw
    /// pos: player position in 3D (x, 0, z)
    /// yaw: player rotation in radians (0 = +X direction)
    pub fn set_first_person(&mut self, pos: Vec3, yaw: f32) {
        // Set eye position at player position + eye height
        self.eye = Vec3::new(pos.x, self.eye_height, pos.z);

        // Calculate forward direction from yaw
        // In 2D: forward = (cos(yaw), sin(yaw))
        // In 3D: forward = (cos(yaw), 0, sin(yaw))
        let forward = Vec3::new(yaw.cos(), 0.0, yaw.sin());

        // Set target to look in forward direction
        self.target = self.eye + forward;
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
