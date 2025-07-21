//! # Application Module
//!
//! This module contains the core application structure and event handling for the Haggis 3D engine.
//! It manages the application lifecycle, window creation, and the main event loop.
//!
//! ## Overview
//!
//! The [`HaggisApp`] is the main entry point for creating and running Haggis applications.
//! It provides a simple, builder-pattern API for configuring 3D scenes, simulations, and UI.
//!
//! ## Key Components
//!
//! - [`HaggisApp`] - Main application struct with builder pattern configuration
//! - [`AppState`] - Internal state management for graphics, UI, and simulation
//! - [`UiCallback`] - Type alias for user-defined UI callback functions
//!
//! ## Event Handling
//!
//! The application implements winit's [`ApplicationHandler`] trait to process:
//! - Window events (resize, close, keyboard input)
//! - Device events (mouse movement for camera controls)
//! - UI events (ImGui interaction)
//! - Simulation updates and rendering
//!
//! ## Usage
//!
//! ```no_run
//! use haggis::HaggisApp;
//!
//! let mut app = HaggisApp::new().await;
//!
//! // Configure scene
//! app.add_object("model.obj")
//!     .with_transform([0.0, 0.0, 0.0], 1.0, 0.0);
//!
//! // Set up UI
//! app.set_ui(|ui, scene, selected| {
//!     ui.window("Debug").build(|| {
//!         ui.text(format!("Objects: {}", scene.objects.len()));
//!     });
//! });
//!
//! // Run the application
//! app.run();
//! ```

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
    simulation::{manager::SimulationManager, traits::Simulation},
    ui::{manager::UiManager, panel::default_transform_panel},
    visualization::{manager::VisualizationManager, traits::VisualizationComponent},
};

/// UI callback function signature for custom user interface rendering.
///
/// This type defines the signature for user-provided UI callback functions that are called
/// every frame during the UI update phase. The callback receives:
///
/// - `ui`: ImGui UI context for drawing interface elements
/// - `scene`: Mutable reference to the 3D scene for object manipulation
/// - `selected_index`: Currently selected object index for UI focus
///
/// # Examples
///
/// ```no_run
/// use haggis::HaggisApp;
///
/// let mut app = HaggisApp::new().await;
/// app.set_ui(|ui, scene, selected_index| {
///     ui.window("Scene Inspector").build(|| {
///         ui.text(format!("Objects: {}", scene.objects.len()));
///         if let Some(index) = selected_index {
///             ui.text(format!("Selected: {}", index));
///         }
///     });
/// });
/// ```
pub type UiCallback = Box<dyn Fn(&imgui::Ui, &mut Scene, &mut Option<usize>) + Send + Sync>;

/// Main Haggis application struct that manages the application lifecycle.
///
/// This is the primary interface for creating and configuring Haggis applications.
/// It provides a builder-pattern API for setting up 3D scenes, simulations, and UI
/// before running the main event loop.
///
/// The application manages:
/// - Window creation and event handling
/// - Graphics rendering pipeline
/// - Simulation execution
/// - User interface rendering
/// - Resource management
///
/// # Examples
///
/// ## Basic Usage
/// ```no_run
/// use haggis;
///
/// let mut app = haggis::default();
/// app.add_object("model.obj")
///     .with_transform([0.0, 0.0, 0.0], 1.0, 0.0);
/// app.run();
/// ```
///
/// ## With Simulation
/// ```no_run
/// use haggis::HaggisApp;
/// use haggis::simulation::traits::Simulation;
///
/// struct MySimulation;
/// impl Simulation for MySimulation {
///     fn update(&mut self, _dt: f32, _scene: &mut haggis::gfx::scene::Scene, _device: Option<&wgpu::Device>, _queue: Option<&wgpu::Queue>) {}
///     fn name(&self) -> &str { "MySimulation" }
///     fn render_ui(&mut self, _ui: &imgui::Ui, _scene: &mut haggis::gfx::scene::Scene) {}
/// }
///
/// let mut app = haggis::default();
/// app.attach_simulation(MySimulation);
/// app.run();
/// ```
pub struct HaggisApp {
    event_loop: Option<EventLoop<()>>,
    /// Application state containing graphics, UI, and simulation components
    pub app_state: AppState,
}

/// Internal application state containing all runtime components.
///
/// This struct holds all the runtime state for the Haggis application, including
/// graphics resources, scene data, UI management, and simulation state.
/// It's separated from [`HaggisApp`] to implement winit's [`ApplicationHandler`] trait.
///
/// # Fields
///
/// - `window`: The application window handle
/// - `render_engine`: Graphics rendering engine with wgpu backend
/// - `ui_manager`: ImGui UI system manager
/// - `scene`: 3D scene containing objects, materials, and camera
/// - `ui_callback`: User-defined UI rendering callback
/// - `selected_object_index`: Currently selected object for UI interaction
/// - `simulation_manager`: Manages CPU/GPU simulations
pub struct AppState {
    window: Option<Arc<Window>>,
    /// Graphics rendering engine
    pub render_engine: Option<RenderEngine>,
    ui_manager: Option<UiManager>,
    /// 3D scene containing objects, materials, and camera
    pub scene: Scene,
    /// User-defined UI callback function
    pub ui_callback: Option<UiCallback>,
    selected_object_index: Option<usize>,
    /// Simulation management system
    pub simulation_manager: SimulationManager,
    /// Visualization management system
    pub visualization_manager: VisualizationManager,
}

impl HaggisApp {
    /// Creates a new Haggis application with default settings.
    ///
    /// Sets up a default orbit camera positioned 8 units from origin,
    /// with reasonable sensitivity and zoom limits. The camera is configured
    /// to orbit around the origin with smooth mouse controls.
    ///
    /// # Returns
    ///
    /// A configured [`HaggisApp`] ready for object addition and UI setup.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use haggis::HaggisApp;
    ///
    /// # async fn example() {
    /// let app = HaggisApp::new().await;
    /// // Configure the app...
    /// # }
    /// ```
    pub async fn new() -> Self {
        let event_loop = EventLoop::new().expect("Failed to create event loop");

        // Configure default orbit camera
        let mut camera = OrbitCamera::new(8.0, 0.4, 0.2, Vector3::new(0.0, 0.0, 0.0), 1.0);
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
                simulation_manager: SimulationManager::new(),
                visualization_manager: VisualizationManager::new(),
            },
        }
    }

    /// Attach a user-defined simulation to the engine.
    ///
    /// This method registers a simulation that will be updated every frame.
    /// The simulation can be either CPU-based or GPU-based, depending on the
    /// implementation of the [`Simulation`] trait.
    ///
    /// # Arguments
    ///
    /// * `simulation` - User simulation implementing the [`Simulation`] trait
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use haggis::HaggisApp;
    /// use haggis::simulation::traits::Simulation;
    ///
    /// struct MyPhysicsSimulation;
    /// impl Simulation for MyPhysicsSimulation {
    ///     fn update(&mut self, _dt: f32, _scene: &mut haggis::gfx::scene::Scene, _device: Option<&wgpu::Device>, _queue: Option<&wgpu::Queue>) {}
    ///     fn name(&self) -> &str { "Physics" }
    ///     fn render_ui(&mut self, _ui: &imgui::Ui, _scene: &mut haggis::gfx::scene::Scene) {}
    /// }
    ///
    /// let mut app = haggis::default();
    /// app.attach_simulation(MyPhysicsSimulation);
    /// ```
    pub fn attach_simulation<T: Simulation + 'static>(&mut self, simulation: T) {
        self.app_state
            .simulation_manager
            .attach_simulation(Box::new(simulation), &mut self.app_state.scene);
    }

    /// Remove the current simulation from the engine.
    ///
    /// This method detaches any currently running simulation and cleans up
    /// its resources. The scene will no longer be updated by simulation code.
    pub fn detach_simulation(&mut self) {
        self.app_state
            .simulation_manager
            .detach_simulation(&mut self.app_state.scene);
    }

    /// Check if a simulation is currently running.
    ///
    /// # Returns
    ///
    /// `true` if a simulation is attached and not paused, `false` otherwise.
    pub fn is_simulation_running(&self) -> bool {
        self.app_state.simulation_manager.is_running()
    }

    /// Get the name of the current simulation.
    ///
    /// # Returns
    ///
    /// An optional reference to the simulation name, or `None` if no simulation is attached.
    pub fn current_simulation(&self) -> Option<&str> {
        self.app_state.simulation_manager.current_simulation_name()
    }

    /// Add a visualization component to the engine.
    ///
    /// This method registers a visualization component that will be updated every frame
    /// and rendered in its own UI panel.
    ///
    /// # Arguments
    ///
    /// * `name` - Unique name for the visualization component
    /// * `component` - The visualization component to add
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use haggis::{HaggisApp, visualization::CutPlane2D};
    ///
    /// let mut app = haggis::default();
    /// let cut_plane = CutPlane2D::new();
    /// app.add_visualization("cut_plane", Box::new(cut_plane));
    /// ```
    pub fn add_visualization<T: VisualizationComponent + 'static>(
        &mut self,
        name: &str,
        component: T,
    ) {
        self.app_state
            .visualization_manager
            .add_component(name.to_string(), Box::new(component));
    }

    /// Remove a visualization component from the engine.
    ///
    /// # Arguments
    ///
    /// * `name` - Name of the visualization component to remove
    pub fn remove_visualization(&mut self, name: &str) {
        self.app_state.visualization_manager.remove_component(name);
    }

    /// Check if the visualization system is enabled.
    ///
    /// # Returns
    ///
    /// `true` if visualizations are enabled, `false` otherwise.
    pub fn is_visualization_enabled(&self) -> bool {
        self.app_state.visualization_manager.is_enabled()
    }

    /// Set the enabled state of the visualization system.
    ///
    /// # Arguments
    ///
    /// * `enabled` - `true` to enable visualizations, `false` to disable them
    pub fn set_visualization_enabled(&mut self, enabled: bool) {
        self.app_state.visualization_manager.set_enabled(enabled);
    }

    /// Sets the UI callback function for custom user interface rendering.
    ///
    /// The callback is called every frame during the UI update phase,
    /// allowing dynamic interface creation and scene manipulation.
    /// This is where you can create custom ImGui windows, controls, and
    /// interactive elements.
    ///
    /// # Arguments
    ///
    /// * `ui_fn` - Function that receives UI context, scene, and selection state
    ///
    /// # Examples
    ///
    /// ## Simple UI
    /// ```no_run
    /// use haggis::HaggisApp;
    ///
    /// let mut app = haggis::default();
    /// app.set_ui(|ui, scene, selected| {
    ///     ui.window("Controls").build(|| {
    ///         ui.text(format!("Objects: {}", scene.objects.len()));
    ///     });
    /// });
    /// ```
    ///
    /// ## Interactive UI
    /// ```no_run
    /// use haggis::HaggisApp;
    ///
    /// let mut app = haggis::default();
    /// app.set_ui(|ui, scene, selected| {
    ///     ui.window("Scene Editor").build(|| {
    ///         if ui.button("Add Object") {
    ///             // Object manipulation logic
    ///         }
    ///         if let Some(index) = selected {
    ///             ui.text(format!("Selected: {}", index));
    ///         }
    ///     });
    /// });
    /// ```
    pub fn set_ui<F>(&mut self, ui_fn: F)
    where
        F: Fn(&imgui::Ui, &mut Scene, &mut Option<usize>) + Send + Sync + 'static,
    {
        self.app_state.ui_callback = Some(Box::new(ui_fn));
    }

    /// Runs the application.
    ///
    /// Consumes the [`HaggisApp`] and starts the main event loop.
    /// This function will block until the application is closed by the user.
    ///
    /// The event loop handles:
    /// - Window events (resize, close, input)
    /// - Graphics rendering
    /// - Simulation updates
    /// - UI rendering
    ///
    /// # Panics
    ///
    /// Panics if the event loop fails to start or if called multiple times.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use haggis::HaggisApp;
    ///
    /// let app = haggis::default();
    /// app.run(); // Blocks until application is closed
    /// ```
    pub fn run(mut self) {
        let event_loop = self.event_loop.take().expect("Event loop already consumed");
        event_loop.set_control_flow(ControlFlow::Poll);

        event_loop
            .run_app(&mut self.app_state)
            .expect("Failed to run event loop");
    }

    /// Adds a 3D object to the scene with builder pattern support.
    ///
    /// Loads a 3D model file and adds it to the scene. Returns an [`ObjectBuilder`]
    /// for method chaining to set transform, materials, and other properties.
    /// The object name is automatically extracted from the file path.
    ///
    /// # Arguments
    ///
    /// * `object_path` - Path to the 3D model file (OBJ format supported)
    ///
    /// # Returns
    ///
    /// An [`ObjectBuilder`] for configuring the added object
    ///
    /// # Examples
    ///
    /// ## Basic Usage
    /// ```no_run
    /// use haggis::HaggisApp;
    ///
    /// let mut app = haggis::default();
    /// app.add_object("models/cube.obj")
    ///     .with_transform([2.0, 0.0, 0.0], 1.5, 45.0);
    /// ```
    ///
    /// ## With Material
    /// ```no_run
    /// use haggis::HaggisApp;
    ///
    /// let mut app = haggis::default();
    /// app.add_object("models/sphere.obj")
    ///     .with_material("gold")
    ///     .with_transform([0.0, 1.0, 0.0], 2.0, 0.0);
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

    /// Adds a 3D object without builder pattern (legacy compatibility).
    ///
    /// This is a simple method for adding objects without the builder pattern.
    /// Use [`add_object`] for more configuration options.
    ///
    /// # Arguments
    ///
    /// * `object_path` - Path to the 3D model file
    ///
    /// [`add_object`]: Self::add_object
    pub fn add_object_simple(&mut self, object_path: &str) {
        self.app_state.scene.add_object(object_path);
    }
}

impl ApplicationHandler for AppState {
    /// Called when the application is resumed or first started.
    ///
    /// This method handles window creation and graphics initialization.
    /// It sets up the wgpu render engine, ImGui UI system, and initializes
    /// GPU resources for the scene and any attached simulations.
    ///
    /// # Arguments
    ///
    /// * `event_loop` - The active event loop for window creation
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

            // Initialize GPU resources for current simulation
            if let Some(render_engine) = &self.render_engine {
                self.simulation_manager
                    .initialize_gpu(render_engine.device(), render_engine.queue());

                // Initialize GPU resources for visualizations
                self.visualization_manager
                    .initialize_gpu(render_engine.device(), render_engine.queue());
            }
        }
    }

    /// Handles window-specific events including input, resizing, and rendering.
    ///
    /// This method processes all window-related events in the following order:
    /// 1. UI input handling (takes precedence to prevent camera interference)
    /// 2. Keyboard input for camera controls and shortcuts
    /// 3. Window resizing and DPI changes
    /// 4. Rendering updates and simulation
    ///
    /// # Arguments
    ///
    /// * `event_loop` - The active event loop
    /// * `window_id` - ID of the window that generated the event
    /// * `event` - The specific window event to handle
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
            WindowEvent::KeyboardInput { event, .. } => {
                // Handle camera keyboard events (like Shift for panning)
                self.scene.camera_manager.process_keyboard_event(&event);

                // Handle other keyboard shortcuts
                if let winit::event::KeyEvent {
                    physical_key: winit::keyboard::PhysicalKey::Code(key_code),
                    state: winit::event::ElementState::Pressed,
                    ..
                } = event
                {
                    if matches!(key_code, winit::keyboard::KeyCode::Escape) {
                        event_loop.exit();
                    }
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
                let Some(render_engine) = self.render_engine.as_mut() else {
                    return;
                };

                // Calculate delta time for simulation
                // Note: You might want to add a proper delta time calculation here
                let delta_time = 0.016; // Approximate 60 FPS - replace with actual timing

                // Update simulation before scene update
                self.simulation_manager.update(
                    delta_time,
                    &mut self.scene,
                    Some(render_engine.device()),
                    Some(render_engine.queue()),
                );

                // Update visualizations (no longer creates scene objects)
                self.visualization_manager.update(
                    delta_time,
                    Some(render_engine.device()),
                    Some(render_engine.queue()),
                );

                // Initialize GPU resources for any new scene objects (but not visualizations)
                self.scene
                    .init_gpu_resources(render_engine.device(), render_engine.queue());

                // Update materials for scene objects (but not visualizations)
                self.scene
                    .update_materials(render_engine.device(), render_engine.queue());

                // Update phase: Scene logic and UI interaction
                self.scene.update();
                if let (Some(ui_manager), Some(ui_callback)) =
                    (self.ui_manager.as_mut(), &self.ui_callback)
                {
                    let ui_wants_input = ui_manager.update_logic(window, |ui| {
                        // Render default object transformation UI (left side)
                        default_transform_panel(
                            ui,
                            &mut self.scene,
                            &mut self.selected_object_index,
                        );

                        // Render simulation UI first
                        self.simulation_manager.render_ui(ui, &mut self.scene);

                        // Render visualization UI (right side)
                        self.visualization_manager.render_ui(ui);

                        // Then render user UI callback if provided
                        ui_callback(ui, &mut self.scene, &mut self.selected_object_index);
                    });

                    // Camera controls are disabled when UI has focus
                    if !ui_wants_input {
                        // Camera input processing would happen here
                    }
                } else if let Some(ui_manager) = self.ui_manager.as_mut() {
                    // If no user UI callback, still render default UI, simulation UI and visualizations
                    let _ui_wants_input = ui_manager.update_logic(window, |ui| {
                        // Render default object transformation UI (left side)
                        default_transform_panel(
                            ui,
                            &mut self.scene,
                            &mut self.selected_object_index,
                        );

                        self.simulation_manager.render_ui(ui, &mut self.scene);
                        self.visualization_manager.render_ui(ui);
                    });
                }

                // Apply UI transform changes to GPU buffers (only if no simulation is controlling objects)
                if let Some(render_engine_ref) = self.render_engine.as_ref() {
                    self.scene
                        .apply_ui_transforms_and_update_gpu(render_engine_ref.queue());
                }

                // Render phase: Draw 3D scene and UI overlay
                let Some(render_engine) = self.render_engine.as_mut() else {
                    return;
                };

                render_engine.update(self.scene.camera_manager.camera.uniform);

                // Collect visualization planes from both the visualization manager and simulation manager
                let mut visualization_planes = self.visualization_manager.get_visualization_planes();
                let simulation_planes = self.simulation_manager.get_visualization_planes();
                visualization_planes.extend(simulation_planes);

                if self.ui_manager.is_some() {
                    // Render 3D scene with visualization planes and UI overlay
                    render_engine.render_frame_with_visualizations_and_ui(
                        &self.scene,
                        &visualization_planes,
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
                    // Render 3D scene with visualization planes only
                    render_engine.render_frame_with_visualizations(&self.scene, &visualization_planes);
                }
            }
            _ => (),
        }
    }
    /// Handles device-level input events such as mouse movement and raw input.
    ///
    /// This method processes camera controls when the UI is not capturing input,
    /// ensuring that camera movement doesn't interfere with UI interactions.
    ///
    /// # Arguments
    ///
    /// * `_event_loop` - The active event loop (unused)
    /// * `_device_id` - ID of the input device (unused)
    /// * `event` - The device-specific event to handle
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

    /// Called when the event loop is about to wait for new events.
    ///
    /// This method requests a redraw to maintain smooth animation and responsive UI.
    /// It ensures continuous rendering for real-time simulations and interactions.
    ///
    /// # Arguments
    ///
    /// * `_event_loop` - The active event loop (unused)
    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(ref window) = self.window {
            window.request_redraw();
        }
    }
}
