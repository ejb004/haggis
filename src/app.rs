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
    ui::UiManager,
};

// UI callback type
pub type UiCallback = Box<dyn Fn(&imgui::Ui) + Send + Sync>;

pub struct HaggisApp {
    event_loop: Option<EventLoop<()>>,
    app_state: AppState,
    ui_callback: Option<UiCallback>,
}

struct AppState {
    window: Option<Arc<Window>>,
    render_engine: Option<RenderEngine>,
    ui_manager: Option<UiManager>,
    scene: Scene,
    ui_callback: Option<UiCallback>,
}

impl HaggisApp {
    /// Create a new Haggis application with default settings
    pub async fn new() -> Self {
        let event_loop = EventLoop::new().expect("Failed to create event loop");

        let mut camera = OrbitCamera::new(5.0, 0.4, 0.2, Vector3::new(0.0, 0.0, 0.0), 1.0);
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
                ui_manager: None,
                ui_callback: None,
            },
            ui_callback: None,
        }
    }

    /// Set UI callback
    pub fn set_ui<F>(&mut self, ui_fn: F)
    where
        F: Fn(&imgui::Ui) + Send + Sync + 'static,
    {
        self.ui_callback = Some(Box::new(ui_fn));
    }

    /// Run the application (consumes self and starts the event loop)
    pub fn run(mut self) {
        // Move UI callback to app_state
        self.app_state.ui_callback = self.ui_callback.take();

        let event_loop = self.event_loop.take().expect("Event loop already consumed");
        event_loop.set_control_flow(ControlFlow::Poll);

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
            return;
        }

        if let Ok(window) = event_loop.create_window(
            WindowAttributes::default().with_inner_size(winit::dpi::LogicalSize::new(1200, 800)),
        ) {
            let window_handle = Arc::new(window);
            self.window = Some(window_handle.clone());

            let (width, height) = window_handle.inner_size().into();

            let window_clone = window_handle.clone();
            let renderer =
                pollster::block_on(
                    async move { RenderEngine::new(window_clone, width, height).await },
                );

            self.scene.init_gpu_resources(renderer.device());

            // Create UI manager
            let ui_manager = UiManager::new(
                renderer.device(),
                renderer.queue(),
                renderer.surface_format(),
                &window_handle,
            );

            self.ui_manager = Some(ui_manager);
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

        let Some(window) = self.window.as_ref() else {
            return;
        };

        // Handle UI input first
        if let Some(ui_manager) = self.ui_manager.as_mut() {
            let ui_event: winit::event::Event<()> = winit::event::Event::WindowEvent {
                window_id,
                event: event.clone(),
            };
            if ui_manager.handle_input(window, &ui_event) {
                // UI consumed the event - request redraw and return early
                window.request_redraw();
                return;
            }
        }

        match event {
            WindowEvent::KeyboardInput {
                event:
                    winit::event::KeyEvent {
                        physical_key: winit::keyboard::PhysicalKey::Code(key_code),
                        ..
                    },
                ..
            } => {
                if matches!(key_code, winit::keyboard::KeyCode::Escape) {
                    event_loop.exit();
                }
                // If you have camera keyboard controls, add the UI check here too:
                // if let Some(ui_manager) = self.ui_manager.as_ref() {
                //     let io = ui_manager.context.io();
                //     if !io.want_capture_keyboard {
                //         self.scene.camera_manager.process_keyboard_event(&event);
                //     }
                // }
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

                // Render with UI if available
                if let (Some(ui_manager), Some(ui_callback)) =
                    (self.ui_manager.as_mut(), &self.ui_callback)
                {
                    let window_clone = window.clone();
                    render_engine.render_frame_with_ui(
                        &self.scene,
                        |device, queue, encoder, color_attachment| {
                            ui_manager.draw(
                                device,
                                queue,
                                encoder,
                                &window_clone,
                                color_attachment,
                                |ui| {
                                    ui_callback(ui);
                                },
                            );
                        },
                    );
                } else {
                    render_engine.render_frame(&self.scene);
                }
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

        // Check if UI wants to capture input before processing camera events
        if let Some(ui_manager) = self.ui_manager.as_ref() {
            let io = ui_manager.context.io();
            if io.want_capture_mouse || io.want_capture_keyboard {
                return; // Don't process camera events when UI is active
            }
        }

        self.scene.camera_manager.process_event(&event, window);
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(ref window) = self.window {
            window.request_redraw();
        }
    }
}
