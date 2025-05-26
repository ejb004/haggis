use cgmath::Vector3;
use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowAttributes},
};

use crate::gfx::{
    camera::{
        camera_controller::CameraController, camera_utils::CameraManager, orbit_camera::OrbitCamera,
    },
    render_engine::RenderEngine,
    scene::Scene,
};

pub struct HaggisApp {
    event_loop: Option<EventLoop<()>>,
    app_state: AppState,
}

struct AppState {
    window: Option<Arc<Window>>,
    render_engine: Option<RenderEngine>,
    scene: Scene,
}

impl HaggisApp {
    /// Create a new Haggis application with default settings
    pub async fn new() -> Self {
        let event_loop = EventLoop::new().expect("Failed to create event loop");

        let mut camera = OrbitCamera::new(2.0, 0.4, 0.2, Vector3::new(0.0, 0.0, 0.0), 1.0);
        camera.bounds.min_distance = Some(1.1);
        let controller = CameraController::new(0.005, 0.1);

        let camera_manager = CameraManager::new(camera, controller);

        let scene = Scene::new(camera_manager);

        Self {
            event_loop: Some(event_loop),
            app_state: AppState {
                window: None,
                render_engine: None,
                scene,
            },
        }
    }
    /// Run the application (consumes self and starts the event loop)
    pub fn run(mut self) {
        let event_loop = self.event_loop.take().expect("Event loop already consumed");
        event_loop.set_control_flow(ControlFlow::Poll);

        // Run the event loop with our application state
        event_loop
            .run_app(&mut self.app_state)
            .expect("Failed to run event loop");
    }

    pub fn add_object(&mut self, object_path: &str) {
        self.app_state.scene.add_object(object_path)
    }
}

impl ApplicationHandler for AppState {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return; // Already initialized
        }

        if let Ok(window) = event_loop.create_window(WindowAttributes::default()) {
            let window_handle = Arc::new(window);
            self.window = Some(window_handle.clone());

            let (width, height) = window_handle.inner_size().into();

            let renderer = pollster::block_on(async move {
                RenderEngine::new(window_handle.clone(), width, height).await
            });

            self.scene.init_gpu_resources(renderer.device());
            self.render_engine = Some(renderer);
        }
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        let Some(render_engine) = self.render_engine.as_mut() else {
            return;
        };

        match event {
            WindowEvent::KeyboardInput {
                event:
                    winit::event::KeyEvent {
                        physical_key: winit::keyboard::PhysicalKey::Code(key_code),
                        ..
                    },
                ..
            } => {
                // Exit by pressing the escape key
                if matches!(key_code, winit::keyboard::KeyCode::Escape) {
                    event_loop.exit();
                }
            }
            WindowEvent::Resized(PhysicalSize { width, height }) => {
                self.scene
                    .camera_manager
                    .camera
                    .resize_projection(width, height);
                render_engine.resize(width, height);
            }
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                self.scene.update();
                render_engine.update(self.scene.camera_manager.camera.uniform);
                render_engine.render_frame(&self.scene);
            }
            _ => (),
        }
    }

    fn device_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        device_id: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    ) {
        let (Some(window)) = (self.window.as_ref()) else {
            return;
        };

        self.scene.camera_manager.process_event(&event, window);
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        // Request redraw every frame
        if let Some(ref window) = self.window {
            window.request_redraw();
        }
    }
}
