use cgmath::{Matrix4, SquareMatrix};
use winit::{
    event::{DeviceEvent, KeyEvent},
    window::Window,
};

use super::{camera_controller::CameraController, orbit_camera::OrbitCamera};

pub struct CameraManager {
    pub camera: OrbitCamera,
    pub controller: CameraController,
}

impl CameraManager {
    pub fn new(camera: OrbitCamera, controller: CameraController) -> Self {
        Self { camera, controller }
    }

    pub fn process_event(&mut self, event: &DeviceEvent, window: &Window) {
        self.controller
            .process_events(event, window, &mut self.camera);
    }

    // Updated method - passes camera reference to controller
    pub fn process_keyboard_event(&mut self, event: &KeyEvent) {
        self.controller
            .process_keyed_events(event, &mut self.camera);
    }

    /// Get the view projection matrix from the camera
    pub fn get_view_proj_matrix(&self) -> cgmath::Matrix4<f32> {
        self.camera.build_view_projection_matrix()
    }
}

pub trait Camera: Sized {
    fn build_view_projection_matrix(&self) -> Matrix4<f32>;
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable, Debug)]
pub struct CameraUniform {
    /// The eye position of the camera in homogenous coordinates.
    ///
    /// Homogenous coordinates are used to fullfill the 16 byte alignment requirement.
    pub view_position: [f32; 4],

    /// Contains the view projection matrix.
    pub view_proj: [[f32; 4]; 4],
}

impl Default for CameraUniform {
    /// Creates a default [CameraUniform].
    fn default() -> Self {
        Self {
            view_position: [0.0; 4],
            view_proj: convert_matrix4_to_array(Matrix4::identity()),
        }
    }
}

pub fn convert_matrix4_to_array(matrix4: Matrix4<f32>) -> [[f32; 4]; 4] {
    let mut result = [[0.0; 4]; 4];

    for i in 0..4 {
        for j in 0..4 {
            result[i][j] = matrix4[i][j];
        }
    }

    result
}
