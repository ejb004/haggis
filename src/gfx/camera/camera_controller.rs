use winit::{
    dpi::PhysicalPosition,
    event::{DeviceEvent, ElementState, KeyEvent, MouseScrollDelta},
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};

use super::orbit_camera::OrbitCamera;

pub struct CameraController {
    pub rotate_speed: f32,
    pub zoom_speed: f32,
    pub pan_speed: f32,
    is_drag_rotate: bool,
    is_shift_held: bool,
    is_mouse_pressed: bool,
}

impl CameraController {
    pub fn new(rotate_speed: f32, zoom_speed: f32) -> Self {
        Self {
            rotate_speed,
            zoom_speed,
            pan_speed: 0.01, // Increased for more noticeable panning
            is_drag_rotate: false,
            is_shift_held: false,
            is_mouse_pressed: false,
        }
    }

    pub fn process_events(
        &mut self,
        event: &DeviceEvent,
        window: &Window,
        camera: &mut OrbitCamera,
    ) {
        match event {
            DeviceEvent::Button {
                button: 0, // Left Mouse Button
                state,
            } => {
                self.is_mouse_pressed = *state == ElementState::Pressed;
            }
            DeviceEvent::MouseWheel { delta, .. } => {
                let scroll_amount = -match delta {
                    MouseScrollDelta::LineDelta(_, scroll) => scroll * 1.0,
                    MouseScrollDelta::PixelDelta(PhysicalPosition { y: scroll, .. }) => {
                        *scroll as f32
                    }
                };
                camera.add_distance(scroll_amount * self.zoom_speed);
                window.request_redraw();
            }
            DeviceEvent::MouseMotion { delta } => {
                if self.is_mouse_pressed {
                    if self.is_shift_held {
                        // SHIFT + DRAG = PAN (move focus point)

                        camera.pan((
                            -delta.0 as f32 * self.pan_speed,
                            delta.1 as f32 * self.pan_speed,
                        ));
                    } else {
                        // NORMAL DRAG = ROTATE (orbit around focus)

                        camera.add_yaw(-delta.0 as f32 * self.rotate_speed);
                        camera.add_pitch(delta.1 as f32 * self.rotate_speed);
                    }
                    window.request_redraw();
                }
            }
            _ => (),
        }
    }

    pub fn process_keyed_events(&mut self, event: &KeyEvent, camera: &mut OrbitCamera) {
        match event {
            KeyEvent {
                physical_key: PhysicalKey::Code(KeyCode::ShiftLeft | KeyCode::ShiftRight),
                state,
                ..
            } => {
                let was_shift_held = self.is_shift_held;
                self.is_shift_held = *state == ElementState::Pressed;

                // Debug output
                if was_shift_held != self.is_shift_held {
                    println!("Shift state changed: {}", self.is_shift_held);
                }
            }
            KeyEvent {
                physical_key: PhysicalKey::Code(KeyCode::KeyC),
                state: ElementState::Pressed,
                ..
            } => {
                // Reset camera when Shift+C is pressed
                if self.is_shift_held {
                    println!("🔄 Resetting camera to default position");
                    camera.reset_to_default();
                }
            }
            _ => (),
        }
    }

    /// Updates the drag mode based on current shift and mouse state
    fn update_drag_mode(&mut self) {
        self.is_drag_rotate = self.is_mouse_pressed && !self.is_shift_held;
    }

    /// Returns true if currently panning
    pub fn is_panning(&self) -> bool {
        self.is_mouse_pressed && self.is_shift_held
    }

    /// Returns true if currently rotating
    pub fn is_rotating(&self) -> bool {
        self.is_drag_rotate
    }

    /// Adjust panning sensitivity
    pub fn set_pan_speed(&mut self, speed: f32) {
        self.pan_speed = speed;
    }
}
