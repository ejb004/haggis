//! Haggis Engine - Main Application Module
//!
//! This module contains the core application structure and event handling for the Haggis 3D engine.
//! Built on top of winit for windowing, wgpu for graphics, and imgui for UI.

use cgmath::Vector3;
use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowAttributes},
};

use crate::{
    gfx::{
        camera::{
            camera_controller::CameraController, camera_utils::CameraManager,
            orbit_camera::OrbitCamera,
        },
        rendering::render_engine::RenderEngine,
        scene::{object::ObjectBuilder, scene::Scene},
    },
    ui::UiManager,
};

/// UI callback function signature
///
/// Provides access to:
/// - `ui`: ImGui UI context for drawing interface elements
/// - `scene`: Mutable reference to the 3D scene for object manipulation
/// - `selected_index`: Currently selected object index for UI focus
pub type UiCallback = Box<dyn Fn(&imgui::Ui, &mut Scene, &mut Option<usize>) + Send + Sync>;

/// Main Haggis application struct
///
/// Manages the application lifecycle, window creation, and rendering loop.
/// Uses the builder pattern for configuration before running.
///
/// # Example
/// ```rust
/// let mut app = haggis::default();
/// app.add_object("model.obj").with_transform([0.0, 0.0, 0.0], 1.0, 0.0);
/// app.set_ui(|ui, scene, selected| {
///     ui.text("Hello, Haggis!");
/// });
/// app.run();
/// ```
pub struct HaggisApp {
    event_loop: Option<EventLoop<()>>,
    pub app_state: AppState,
}

/// Internal application state
///
/// Contains all runtime state including graphics resources, scene data,
/// and UI management. Separated from HaggisApp to implement ApplicationHandler.
pub struct AppState {
    window: Option<Arc<Window>>,
    pub render_engine: Option<RenderEngine>,
    ui_manager: Option<UiManager>,
    pub scene: Scene,
    pub ui_callback: Option<UiCallback>,
    selected_object_index: Option<usize>,
}

impl HaggisApp {
    /// Creates a new Haggis application with default settings
    ///
    /// Sets up a default orbit camera positioned 5 units from origin,
    /// with reasonable sensitivity and zoom limits.
    ///
    /// # Returns
    /// A configured HaggisApp ready for object addition and UI setup
    pub async fn new() -> Self {
        let event_loop = EventLoop::new().expect("Failed to create event loop");

        // Configure default orbit camera
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
                selected_object_index: Some(0),
            },
        }
    }

    /// Sets the UI callback function
    ///
    /// The callback is called every frame during the UI update phase,
    /// allowing dynamic interface creation and scene manipulation.
    ///
    /// # Arguments
    /// * `ui_fn` - Function that receives UI context, scene, and selection state
    ///
    /// # Example
    /// ```rust
    /// app.set_ui(|ui, scene, selected| {
    ///     ui.window("Controls").build(|| {
    ///         ui.text("Object count: {}", scene.objects.len());
    ///     });
    /// });
    /// ```
    pub fn set_ui<F>(&mut self, ui_fn: F)
    where
        F: Fn(&imgui::Ui, &mut Scene, &mut Option<usize>) + Send + Sync + 'static,
    {
        self.app_state.ui_callback = Some(Box::new(ui_fn));
    }

    /// Runs the application
    ///
    /// Consumes the HaggisApp and starts the main event loop.
    /// This function will block until the application is closed.
    ///
    /// # Panics
    /// Panics if the event loop fails to start or if called multiple times
    pub fn run(mut self) {
        let event_loop = self.event_loop.take().expect("Event loop already consumed");
        event_loop.set_control_flow(ControlFlow::Poll);

        event_loop
            .run_app(&mut self.app_state)
            .expect("Failed to run event loop");
    }

    /// Adds a 3D object to the scene with builder pattern
    ///
    /// Returns an ObjectBuilder for method chaining to set transform,
    /// materials, and other properties.
    ///
    /// # Arguments
    /// * `object_path` - Path to the 3D model file (OBJ format supported)
    ///
    /// # Returns
    /// ObjectBuilder for configuring the added object
    ///
    /// # Example
    /// ```rust
    /// app.add_object("models/cube.obj")
    ///     .with_transform([2.0, 0.0, 0.0], 1.5, 45.0);
    /// ```
    pub fn add_object(&mut self, object_path: &str) -> ObjectBuilder {
        let object_index = self.app_state.scene.objects.len();
        self.app_state.scene.add_object(object_path);

        // Extract object name from file path for UI display
        if let Some(object) = self.app_state.scene.objects.get_mut(object_index) {
            let object_name = std::path::Path::new(object_path)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Object")
                .to_string();
            object.set_name(object_name);
            object.sync_transform_to_ui();
        }

        ObjectBuilder::new(self, object_index)
    }

    /// Adds a 3D object without builder pattern (legacy compatibility)
    ///
    /// # Arguments
    /// * `object_path` - Path to the 3D model file
    pub fn add_object_simple(&mut self, object_path: &str) {
        self.app_state.scene.add_object(object_path);
    }
}

impl ApplicationHandler for AppState {
    /// Called when the application is resumed or first started
    ///
    /// Handles window creation and graphics initialization.
    /// Sets up the wgpu render engine and ImGui UI system.

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        if let Ok(window) = event_loop.create_window(
            WindowAttributes::default().with_inner_size(winit::dpi::LogicalSize::new(1200, 800)),
        ) {
            let window_handle = Arc::new(window);
            self.window = Some(window_handle.clone());

            // Use physical size for render engine, not logical size
            let physical_size = window_handle.inner_size();
            let (width, height) = (physical_size.width, physical_size.height);

            println!("Window created - Physical size: {}x{}", width, height);
            println!("Window scale factor: {}", window_handle.scale_factor());

            let window_clone = window_handle.clone();
            let renderer =
                pollster::block_on(
                    async move { RenderEngine::new(window_clone, width, height).await },
                );

            // Initialize scene GPU resources (objects)
            self.scene
                .init_gpu_resources(renderer.device(), renderer.queue());

            // Update all transforms after GPU initialization
            self.scene.update_all_transforms(renderer.queue());

            // Force material update if needed
            self.scene
                .update_materials(renderer.device(), renderer.queue());

            // Create UI manager with correct surface dimensions
            let mut ui_manager = UiManager::new(
                renderer.device(),
                renderer.queue(),
                renderer.surface_format(),
                &window_handle,
            );

            // Set ImGui display size to match actual surface size
            let (surface_width, surface_height) = renderer.get_surface_size();
            ui_manager.update_display_size(surface_width, surface_height);

            self.ui_manager = Some(ui_manager);
            self.render_engine = Some(renderer);
        }
    }

    /// Handles window-specific events
    ///
    /// Processes input, window resizing, and triggers rendering.
    /// UI input is handled first to prevent camera movement when interacting with interface.
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

        // UI input handling takes precedence over camera controls
        if let Some(ui_manager) = self.ui_manager.as_mut() {
            let ui_event: winit::event::Event<()> = winit::event::Event::WindowEvent {
                window_id,
                event: event.clone(),
            };
            if ui_manager.handle_input(window, &ui_event) {
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
            }
            WindowEvent::Resized(PhysicalSize { width, height }) => {
                // Update all systems to handle new window size
                self.scene
                    .camera_manager
                    .camera
                    .resize_projection(width, height);
                render_engine.resize(width, height);

                // Keep UI scaling synchronized with actual surface size
                if let Some(ui_manager) = self.ui_manager.as_mut() {
                    let (actual_width, actual_height) = render_engine.get_surface_size();
                    ui_manager.update_display_size(actual_width, actual_height);
                }
            }
            WindowEvent::ScaleFactorChanged {
                scale_factor: _, ..
            } => {
                let PhysicalSize { width, height } = window.inner_size();

                // Handle high-DPI display changes
                self.scene
                    .camera_manager
                    .camera
                    .resize_projection(width, height);
                render_engine.resize(width, height);

                if let Some(ui_manager) = self.ui_manager.as_mut() {
                    let (actual_width, actual_height) = render_engine.get_surface_size();
                    ui_manager.update_display_size(actual_width, actual_height);
                }
            }
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                let Some(_) = self.render_engine.as_ref() else {
                    return;
                };

                // Update phase: Scene logic and UI interaction
                self.scene.update();

                if let (Some(ui_manager), Some(ui_callback)) =
                    (self.ui_manager.as_mut(), &self.ui_callback)
                {
                    let ui_wants_input = ui_manager.update_logic(window, |ui| {
                        ui_callback(ui, &mut self.scene, &mut self.selected_object_index);
                    });

                    // Camera controls are disabled when UI has focus
                    if !ui_wants_input {
                        // Camera input processing would happen here
                    }
                }

                // Apply UI transform changes to GPU buffers
                if let Some(render_engine_ref) = self.render_engine.as_ref() {
                    self.scene
                        .apply_ui_transforms_and_update_gpu(render_engine_ref.queue());
                }

                // Render phase: Draw 3D scene and UI overlay
                let Some(render_engine) = self.render_engine.as_mut() else {
                    return;
                };

                render_engine.update(self.scene.camera_manager.camera.uniform);

                if self.ui_manager.is_some() {
                    // Render 3D scene with UI overlay
                    render_engine.render_frame_with_ui(
                        &self.scene,
                        |device, queue, encoder, color_attachment| {
                            self.ui_manager.as_mut().unwrap().render_display_only(
                                device,
                                queue,
                                encoder,
                                window,
                                color_attachment,
                            );
                        },
                    );
                } else {
                    // Render 3D scene only
                    render_engine.render_frame(&self.scene);
                }
            }
            _ => (),
        }
    }

    /// Handles device-level input events (mouse movement, etc.)
    ///
    /// Processes camera controls when UI is not capturing input.
    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    ) {
        let Some(window) = self.window.as_ref() else {
            return;
        };

        // Respect UI input capture to prevent camera movement during UI interaction
        if let Some(ui_manager) = self.ui_manager.as_ref() {
            let io = ui_manager.context.io();
            if io.want_capture_mouse || io.want_capture_keyboard {
                return;
            }
        }

        self.scene.camera_manager.process_event(&event, window);
    }

    /// Called when the event loop is about to wait for new events
    ///
    /// Requests a redraw to maintain smooth animation and responsive UI.
    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(ref window) = self.window {
            window.request_redraw();
        }
    }
}
