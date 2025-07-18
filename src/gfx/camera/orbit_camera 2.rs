use super::camera_utils::{convert_matrix4_to_array, Camera, CameraUniform};
use cgmath::*;

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.5,
    0.0, 0.0, 0.0, 1.0,
);

#[derive(Debug, Clone, Copy)]
pub struct OrbitCamera {
    pub distance: f32,
    pub pitch: f32,
    pub yaw: f32,
    pub eye: Vector3<f32>,
    pub target: Vector3<f32>,
    pub up: Vector3<f32>,
    pub bounds: OrbitCameraBounds,
    pub aspect: f32,
    pub fovy: Rad<f32>,
    pub znear: f32,
    pub zfar: f32,
    pub uniform: CameraUniform,
}

impl Camera for OrbitCamera {
    fn build_view_projection_matrix(&self) -> Matrix4<f32> {
        let eye = Point3::from_vec(self.eye);
        let target = Point3::from_vec(self.target);
        let view = Matrix4::look_at_rh(eye, target, self.up);
        let proj =
            OPENGL_TO_WGPU_MATRIX * perspective(self.fovy, self.aspect, self.znear, self.zfar);
        proj * view
    }
}

impl OrbitCamera {
    pub fn new(distance: f32, pitch: f32, yaw: f32, target: Vector3<f32>, aspect: f32) -> Self {
        let mut camera = Self {
            distance,
            pitch,
            yaw,
            eye: Vector3::zero(), // Will be auto-calculted in `update()` nevertheless.
            target,
            up: Vector3::unit_z(),
            bounds: OrbitCameraBounds::default(),
            aspect,
            fovy: cgmath::Rad(std::f32::consts::PI / 4.0),
            znear: 0.1,
            zfar: 1000.0,
            uniform: CameraUniform::default(),
        };
        camera.update();
        camera
    }

    pub fn reset_to_default(&mut self) {
        // Store original values - you can customize these defaults
        self.distance = 8.0; // Default distance
        self.pitch = 0.4; // Slight downward angle
        self.yaw = 0.2; // Facing forward
        self.target = Vector3::zero(); // Look at origin

        self.update(); // Recalculate eye position
    }

    pub fn set_distance(&mut self, distance: f32) {
        self.distance = distance.clamp(
            self.bounds.min_distance.unwrap_or(f32::EPSILON),
            self.bounds.max_distance.unwrap_or(f32::MAX),
        );
        self.update();
    }

    pub fn add_distance(&mut self, delta: f32) {
        let corrected_zoom = f32::log10(self.distance) * delta;
        self.set_distance(self.distance + corrected_zoom);
        // println!("{:}", self.distance)
    }

    pub fn set_pitch(&mut self, pitch: f32) {
        self.pitch = pitch.clamp(self.bounds.min_pitch, self.bounds.max_pitch);
        self.update();
    }

    pub fn add_pitch(&mut self, delta: f32) {
        self.set_pitch(self.pitch + delta);
    }

    pub fn set_yaw(&mut self, yaw: f32) {
        let mut bounded_yaw = yaw;
        if let Some(min_yaw) = self.bounds.min_yaw {
            bounded_yaw = bounded_yaw.clamp(min_yaw, f32::MAX);
        }
        if let Some(max_yaw) = self.bounds.max_yaw {
            bounded_yaw = bounded_yaw.clamp(f32::MIN, max_yaw);
        }
        self.yaw = bounded_yaw;
        self.update();
    }

    pub fn add_yaw(&mut self, delta: f32) {
        self.set_yaw(self.yaw + delta);
    }

    /// Pans the camera relative to the current view direction
    /// delta.0 = horizontal pan (left/right relative to camera view)
    /// delta.1 = vertical pan (up/down relative to camera view)
    pub fn pan(&mut self, delta: (f32, f32)) {
        // Calculate camera's local coordinate system
        let forward = (self.target - self.eye).normalize();
        let right = forward.cross(self.up).normalize();
        let up = right.cross(forward).normalize(); // True "up" relative to camera

        // Scale pan movement by distance for consistent feel at all zoom levels
        let pan_scale = self.distance * 0.1; // Adjust this multiplier for sensitivity

        // Calculate movement in view space
        let horizontal_movement = right * delta.0 * pan_scale;
        let vertical_movement = up * delta.1 * pan_scale;
        let total_movement = horizontal_movement + vertical_movement;

        // Move both eye and target to maintain the view direction
        self.eye += total_movement;
        self.target += total_movement;

        // Debug output (remove if not needed)
        // println!("Pan delta: ({:.3}, {:.3}), Movement: {:?}", delta.0, delta.1, total_movement);
    }

    /// Updates the camera after changing `distance`, `pitch` or `yaw`.
    fn update(&mut self) {
        self.eye =
            calculate_cartesian_eye_position(self.pitch, self.yaw, self.distance, self.target);
    }

    pub fn resize_projection(&mut self, width: u32, height: u32) {
        self.aspect = width as f32 / height as f32;
    }

    pub fn update_view_proj(&mut self) {
        self.uniform.view_position = [self.eye.x, self.eye.y, self.eye.z, 1.0];
        self.uniform.view_proj = convert_matrix4_to_array(self.build_view_projection_matrix());
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
#[derive(Debug, Clone, Copy)]
pub struct OrbitCameraBounds {
    pub min_distance: Option<f32>,
    pub max_distance: Option<f32>,
    pub min_pitch: f32,
    pub max_pitch: f32,
    pub min_yaw: Option<f32>,
    pub max_yaw: Option<f32>,
}

impl Default for OrbitCameraBounds {
    fn default() -> Self {
        Self {
            min_distance: None,
            max_distance: Some(16.0),
            min_pitch: -std::f32::consts::PI / 2.0 + f32::EPSILON,
            max_pitch: std::f32::consts::PI / 2.0 - f32::EPSILON,
            min_yaw: None,
            max_yaw: None,
        }
    }
}

fn calculate_cartesian_eye_position(
    pitch: f32,
    yaw: f32,
    distance: f32,
    target: Vector3<f32>,
) -> Vector3<f32> {
    return Vector3::new(
        distance * yaw.sin() * pitch.cos(),
        distance * pitch.sin(),
        distance * yaw.cos() * pitch.cos(),
    ) + target;
}
