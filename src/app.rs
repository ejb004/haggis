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
        picking::ObjectPicker,
        rendering::render_engine::RenderEngine,
        scene::{object::ObjectBuilder, scene::Scene},
    },
    performance::PerformanceMonitor,
    simulation::{manager::SimulationManager, traits::Simulation},
    ui::{manager::UiManager, panel::default_transform_panel, UiFont, UiStyle},
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
    /// UI style configuration
    pub ui_style: UiStyle,
    /// UI font configuration
    pub ui_font: UiFont,
    /// Whether to show the default transform panel
    pub show_transform_panel: bool,
    /// 3D scene containing objects, materials, and camera
    pub scene: Scene,
    /// User-defined UI callback function
    pub ui_callback: Option<UiCallback>,
    selected_object_index: Option<usize>,
    /// Simulation management system
    pub simulation_manager: SimulationManager,
    /// Visualization management system
    pub visualization_manager: VisualizationManager,
    /// Gizmo management system
    pub gizmo_manager: crate::gfx::gizmos::GizmoManager,
    /// Performance monitoring system
    pub performance_monitor: PerformanceMonitor,
    /// Whether to show the performance metrics panel
    pub show_performance_panel: bool,
    /// Enable VSync for smoother visuals vs higher FPS
    pub enable_vsync: bool,
    /// Framerate limit (None = unlimited, Some(fps) = limited)
    pub framerate_limit: Option<f32>,
    /// Frame timing for FPS limiting
    last_frame_time: std::time::Instant,
    /// Frame timing for performance monitoring (tracks actual frame cycle)
    last_performance_frame_time: std::time::Instant,
    /// Object picker for mouse selection
    pub object_picker: ObjectPicker,
    /// Current mouse position for picking
    mouse_position: (f32, f32),
    /// Whether UI captured input in the last frame
    ui_wants_input: bool,
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
                ui_style: UiStyle::default(),
                ui_font: UiFont::default(),
                show_transform_panel: true,
                ui_callback: None,
                selected_object_index: Some(0),
                simulation_manager: SimulationManager::new(),
                visualization_manager: VisualizationManager::new(),
                gizmo_manager: crate::gfx::gizmos::GizmoManager::new(),
                performance_monitor: PerformanceMonitor::new(),
                show_performance_panel: false, // Hidden by default
                enable_vsync: false, // Disabled when framerate limiting is enabled
                framerate_limit: Some(144.0), // Higher limit to ensure we hit 120fps target
                last_frame_time: std::time::Instant::now(),
                last_performance_frame_time: std::time::Instant::now(),
                object_picker: ObjectPicker::new(),
                mouse_position: (0.0, 0.0),
                ui_wants_input: false,
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

    /// Add a gizmo to the engine.
    ///
    /// Gizmos are visual aids that help with debugging, visualization, and interaction
    /// in 3D space. They can represent positions, orientations, paths, bounds, and more.
    ///
    /// # Arguments
    ///
    /// * `name` - Unique identifier for the gizmo
    /// * `gizmo` - The gizmo instance to add
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use haggis::{HaggisApp, gfx::gizmos::CameraGizmo};
    ///
    /// let mut app = haggis::default();
    /// let camera_gizmo = CameraGizmo::new();
    /// app.add_gizmo("camera", camera_gizmo);
    /// ```
    pub fn add_gizmo<T: crate::gfx::gizmos::Gizmo + 'static>(
        &mut self,
        name: &str,
        gizmo: T,
    ) {
        if let (Some(render_engine), _) = (&self.app_state.render_engine, &self.app_state.window) {
            self.app_state.gizmo_manager.add_gizmo(
                name.to_string(),
                Box::new(gizmo),
                &mut self.app_state.scene,
                Some(render_engine.device()),
                Some(render_engine.queue()),
            );
        } else {
            // If render engine isn't initialized yet, we'll need to defer this
            // For now, just add without GPU resources
            self.app_state.gizmo_manager.add_gizmo(
                name.to_string(),
                Box::new(gizmo),
                &mut self.app_state.scene,
                None,
                None,
            );
        }
    }

    /// Remove a gizmo from the engine.
    ///
    /// # Arguments
    ///
    /// * `name` - Name of the gizmo to remove
    pub fn remove_gizmo(&mut self, name: &str) {
        self.app_state.gizmo_manager.remove_gizmo(name, &mut self.app_state.scene);
    }

    /// Check if the gizmo system is enabled.
    ///
    /// # Returns
    ///
    /// `true` if gizmos are enabled, `false` otherwise.
    pub fn is_gizmo_enabled(&self) -> bool {
        self.app_state.gizmo_manager.is_enabled()
    }

    /// Set the enabled state of the gizmo system.
    ///
    /// # Arguments
    ///
    /// * `enabled` - `true` to enable gizmos, `false` to disable them
    pub fn set_gizmo_enabled(&mut self, enabled: bool) {
        self.app_state.gizmo_manager.set_enabled(enabled);
    }

    /// Sets the UI style theme for the application.
    ///
    /// This method configures the global UI appearance using predefined or custom themes.
    /// The style is applied when the UI manager is initialized.
    ///
    /// # Arguments
    ///
    /// * `style` - UI style configuration (Default, Light, Dark, Matrix, or Custom)
    ///
    /// # Examples
    ///
    /// ## Light Theme
    /// ```no_run
    /// use haggis::{HaggisApp, UiStyle};
    ///
    /// let mut app = haggis::default();
    /// app.set_ui_style(UiStyle::Light);
    /// ```
    ///
    /// ## Matrix Theme
    /// ```no_run
    /// use haggis::{HaggisApp, UiStyle};
    ///
    /// let mut app = haggis::default();
    /// app.set_ui_style(UiStyle::Matrix);
    /// ```
    ///
    /// ## Custom Theme
    /// ```no_run
    /// use haggis::{HaggisApp, UiStyle};
    ///
    /// let mut app = haggis::default();
    /// app.set_ui_style(UiStyle::Custom {
    ///     background: [0.2, 0.3, 0.4, 1.0],
    ///     text: [1.0, 1.0, 1.0, 1.0],
    ///     button: [0.4, 0.6, 0.8, 1.0],
    ///     button_hovered: [0.5, 0.7, 0.9, 1.0],
    ///     button_active: [0.3, 0.5, 0.7, 1.0],
    /// });
    /// ```
    pub fn set_ui_style(&mut self, style: UiStyle) {
        self.app_state.ui_style = style;
    }

    /// Sets the UI font configuration for the application.
    ///
    /// This method configures the global UI font. The font is applied when
    /// the UI manager is initialized.
    ///
    /// # Arguments
    ///
    /// * `font` - UI font configuration (Default, Custom, or Monospace)
    ///
    /// # Examples
    ///
    /// ## Default Font
    /// ```no_run
    /// use haggis::{HaggisApp, UiFont};
    ///
    /// let mut app = haggis::default();
    /// app.set_ui_font(UiFont::Default);
    /// ```
    ///
    /// ## Custom Font
    /// ```no_run
    /// use haggis::{HaggisApp, UiFont};
    ///
    /// let mut app = haggis::default();
    /// app.set_ui_font(UiFont::Custom {
    ///     data: include_bytes!("../fonts/my_font.ttf"),
    ///     size: 18.0,
    /// });
    /// ```
    ///
    /// ## Monospace Font
    /// ```no_run
    /// use haggis::{HaggisApp, UiFont};
    ///
    /// let mut app = haggis::default();
    /// app.set_ui_font(UiFont::Monospace);
    /// ```
    pub fn set_ui_font(&mut self, font: UiFont) {
        self.app_state.ui_font = font;
    }

    /// Sets whether to show the default transform panel.
    ///
    /// The transform panel allows editing object position, rotation, and scale
    /// through the UI. It's enabled by default but can be disabled if you want
    /// a cleaner interface or handle transforms through custom UI.
    ///
    /// # Arguments
    ///
    /// * `show` - `true` to show the transform panel, `false` to hide it
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use haggis::HaggisApp;
    ///
    /// let mut app = haggis::default();
    /// // Hide the default transform panel for a cleaner UI
    /// app.set_transform_panel_visible(false);
    /// ```
    pub fn set_transform_panel_visible(&mut self, show: bool) {
        self.app_state.show_transform_panel = show;
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

    /// Enable or disable the performance metrics panel.
    ///
    /// When enabled, a performance metrics panel will be displayed showing:
    /// - Current FPS and frame time
    /// - Frame time statistics (min/max/average)
    /// - Render statistics (draw calls, vertex count)
    /// - Frame time history graph
    ///
    /// # Arguments
    ///
    /// * `enabled` - Whether to show the performance panel
    ///
    /// # Examples
    ///
    /// ```rust
    /// let mut app = haggis::default();
    /// app.show_performance_panel(true); // Enable performance monitoring
    /// app.run();
    /// ```
    pub fn show_performance_panel(&mut self, enabled: bool) {
        self.app_state.show_performance_panel = enabled;
    }


    /// Set framerate limit to prioritize simulation over rendering.
    ///
    /// Limits the maximum framerate to free up resources for simulation computation.
    /// Use None for unlimited framerate, or Some(fps) to set a specific limit.
    /// 
    /// # Arguments
    /// * `limit` - Framerate limit in FPS (None for unlimited)
    ///
    /// # Examples
    /// ```rust
    /// let mut app = haggis::default();
    /// app.set_framerate_limit(Some(120.0)); // Limit to 120 FPS
    /// app.set_framerate_limit(None);        // Unlimited FPS
    /// ```
    pub fn set_framerate_limit(&mut self, limit: Option<f32>) {
        self.app_state.framerate_limit = limit;
    }

    /// Set VSync (vertical synchronization) state.
    ///
    /// When VSync is enabled, the application will sync to the display refresh rate.
    /// When disabled with framerate limiting, the application can achieve consistent
    /// frame times regardless of display refresh rate.
    ///
    /// # Arguments
    /// * `enable` - Whether to enable VSync
    ///
    /// # Examples
    /// ```rust
    /// let mut app = haggis::default();
    /// app.set_vsync(false); // Disable VSync for consistent framerate limiting
    /// ```
    pub fn set_vsync(&mut self, enable: bool) {
        self.app_state.enable_vsync = enable;
        
        // Update render engine surface configuration if available
        if let Some(render_engine) = &mut self.app_state.render_engine {
            render_engine.set_vsync(enable);
        }
    }

    /// Get the current performance metrics.
    ///
    /// Returns a reference to the current performance metrics which include
    /// FPS, frame time, memory usage, and render statistics.
    ///
    /// # Returns
    ///
    /// A reference to the current [`PerformanceMetrics`](crate::performance::PerformanceMetrics).
    ///
    /// # Examples
    ///
    /// ```rust
    /// let app = haggis::default();
    /// let metrics = app.get_performance_metrics();
    /// println!("Current FPS: {:.1}", metrics.fps);
    /// ```
    pub fn get_performance_metrics(&self) -> &crate::performance::PerformanceMetrics {
        self.app_state.performance_monitor.get_metrics()
    }

    /// Reset performance metrics and history.
    ///
    /// This clears all accumulated performance data and restarts tracking
    /// from the current frame. Useful for benchmarking specific scenarios.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let mut app = haggis::default();
    /// // ... run for a while ...
    /// app.reset_performance_metrics(); // Start fresh
    /// ```
    pub fn reset_performance_metrics(&mut self) {
        self.app_state.performance_monitor.reset();
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

    /// Add a procedural cube to the scene.
    ///
    /// Creates a unit cube (1x1x1) centered at the origin with proper normals and texture coordinates.
    /// The cube can be scaled and positioned using the returned [`ObjectBuilder`].
    ///
    /// # Returns
    ///
    /// An [`ObjectBuilder`] for further configuration of the cube.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let mut app = haggis::default();
    /// 
    /// // Add a simple cube
    /// app.add_cube();
    ///
    /// // Add a cube with custom properties
    /// app.add_cube()
    ///     .with_name("My Cube")
    ///     .with_material("red")
    ///     .with_transform([0.0, 0.0, 2.0], 0.5, 45.0);
    /// ```
    pub fn add_cube(&mut self) -> ObjectBuilder {
        let object_index = self.app_state.scene.objects.len();
        let cube_geometry = crate::gfx::geometry::generate_cube();
        self.app_state.scene.add_procedural_object(cube_geometry, "Cube");

        // Sync transform for UI
        if let Some(object) = self.app_state.scene.objects.get_mut(object_index) {
            object.sync_transform_to_ui();
        }

        ObjectBuilder::new(self, object_index)
    }

    /// Add a procedural sphere to the scene.
    ///
    /// Creates a UV sphere with the specified resolution. Higher values create smoother spheres
    /// but use more vertices.
    ///
    /// # Arguments
    ///
    /// * `longitude_segments` - Number of vertical segments (longitude lines). Default: 32
    /// * `latitude_segments` - Number of horizontal segments (latitude lines). Default: 16
    ///
    /// # Returns
    ///
    /// An [`ObjectBuilder`] for further configuration of the sphere.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let mut app = haggis::default();
    /// 
    /// // Add a smooth sphere
    /// app.add_sphere(32, 16)
    ///     .with_name("Smooth Sphere")
    ///     .with_material("blue");
    ///
    /// // Add a low-poly sphere
    /// app.add_sphere(8, 6)
    ///     .with_name("Low Poly Sphere")
    ///     .with_transform([2.0, 0.0, 0.0], 1.0, 0.0);
    /// ```
    pub fn add_sphere(&mut self, longitude_segments: u32, latitude_segments: u32) -> ObjectBuilder {
        let object_index = self.app_state.scene.objects.len();
        let sphere_geometry = crate::gfx::geometry::generate_sphere(longitude_segments, latitude_segments);
        self.app_state.scene.add_procedural_object(sphere_geometry, "Sphere");

        // Sync transform for UI
        if let Some(object) = self.app_state.scene.objects.get_mut(object_index) {
            object.sync_transform_to_ui();
        }

        ObjectBuilder::new(self, object_index)
    }

    /// Add a procedural plane to the scene.
    ///
    /// Creates a plane in the XY plane (horizontal in Z-up coordinate system) with the specified
    /// dimensions and subdivision level.
    ///
    /// # Arguments
    ///
    /// * `width` - Width of the plane (X direction)
    /// * `height` - Height of the plane (Y direction)
    /// * `width_segments` - Number of subdivisions along width. Default: 1
    /// * `height_segments` - Number of subdivisions along height. Default: 1
    ///
    /// # Returns
    ///
    /// An [`ObjectBuilder`] for further configuration of the plane.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let mut app = haggis::default();
    /// 
    /// // Add a simple ground plane
    /// app.add_plane(10.0, 10.0, 1, 1)
    ///     .with_name("Ground")
    ///     .with_material("grass");
    ///
    /// // Add a subdivided plane for deformation
    /// app.add_plane(5.0, 5.0, 8, 8)
    ///     .with_name("Subdivided Plane")
    ///     .with_transform([0.0, 0.0, 0.0], 1.0, 0.0);
    /// ```
    pub fn add_plane(&mut self, width: f32, height: f32, width_segments: u32, height_segments: u32) -> ObjectBuilder {
        let object_index = self.app_state.scene.objects.len();
        let plane_geometry = crate::gfx::geometry::generate_plane(width, height, width_segments, height_segments);
        self.app_state.scene.add_procedural_object(plane_geometry, "Plane");

        // Sync transform for UI
        if let Some(object) = self.app_state.scene.objects.get_mut(object_index) {
            object.sync_transform_to_ui();
        }

        ObjectBuilder::new(self, object_index)
    }

    /// Add a procedural cylinder to the scene.
    ///
    /// Creates a cylinder with the specified radius, height, and number of segments.
    /// The cylinder extends from -height/2 to height/2 along the Z-axis.
    ///
    /// # Arguments
    ///
    /// * `radius` - Radius of the cylinder
    /// * `height` - Height of the cylinder (along Z-axis)
    /// * `segments` - Number of circular segments. Higher values create smoother cylinders
    ///
    /// # Returns
    ///
    /// An [`ObjectBuilder`] for further configuration of the cylinder.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let mut app = haggis::default();
    /// 
    /// // Add a smooth cylinder
    /// app.add_cylinder(1.0, 2.0, 32)
    ///     .with_name("Pillar")
    ///     .with_material("stone");
    ///
    /// // Add a low-poly cylinder
    /// app.add_cylinder(0.5, 1.5, 6)
    ///     .with_name("Hex Pillar")
    ///     .with_transform([3.0, 0.0, 0.0], 1.0, 0.0);
    /// ```
    pub fn add_cylinder(&mut self, radius: f32, height: f32, segments: u32) -> ObjectBuilder {
        let object_index = self.app_state.scene.objects.len();
        let cylinder_geometry = crate::gfx::geometry::generate_cylinder(radius, height, segments);
        self.app_state.scene.add_procedural_object(cylinder_geometry, "Cylinder");

        // Sync transform for UI
        if let Some(object) = self.app_state.scene.objects.get_mut(object_index) {
            object.sync_transform_to_ui();
        }

        ObjectBuilder::new(self, object_index)
    }

    /// Initialize the instanced grid system for high-performance rendering
    /// 
    /// This should be called once during app setup if you plan to use instanced grid rendering.
    /// The instanced grid system allows rendering thousands of identical objects efficiently.
    ///
    /// # Arguments
    /// * `max_instances` - Maximum number of instances that can be rendered simultaneously
    pub fn initialize_instanced_grid(&mut self, max_instances: u32) {
        if let Some(ref mut render_engine) = self.app_state.render_engine {
            render_engine.initialize_instanced_grid(max_instances);
            println!("🎲 Initialized instanced grid renderer (max {} instances)", max_instances);
        }
    }

    /// Update the instanced grid with new instance data
    ///
    /// Updates the GPU buffers with new instance data for efficient rendering.
    /// Each instance is defined by position, scale, and color.
    ///
    /// # Arguments
    /// * `instances` - Vector of (position, scale, color) tuples for each instance
    pub fn update_instanced_grid(&mut self, instances: &[(cgmath::Vector3<f32>, f32, cgmath::Vector4<f32>)]) {
        if let Some(ref mut render_engine) = self.app_state.render_engine {
            render_engine.update_instanced_grid_data(instances);
        }
    }

    /// Enable or disable the instanced grid rendering
    ///
    /// # Arguments
    /// * `enabled` - Whether to render the instanced grid
    pub fn set_instanced_grid_enabled(&mut self, enabled: bool) {
        if let Some(ref mut render_engine) = self.app_state.render_engine {
            if let Some(grid) = render_engine.instanced_grid_mut() {
                grid.set_enabled(enabled);
            }
        }
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

            #[cfg(debug_assertions)]
            {
                println!("Window created - Physical size: {}x{}", width, height);
                println!("Window scale factor: {}", window_handle.scale_factor());
            }

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

            // Create UI manager with correct surface dimensions, style, and font
            let mut ui_manager = UiManager::new(
                renderer.device(),
                renderer.queue(),
                renderer.surface_format(),
                &window_handle,
                self.ui_style,
                self.ui_font.clone(),
            );

            // Set ImGui display size to match actual surface size
            let (surface_width, surface_height) = renderer.get_surface_size();
            ui_manager.update_display_size(surface_width, surface_height);

            self.ui_manager = Some(ui_manager);
            self.render_engine = Some(renderer);

            // Configure VSync based on initial settings
            if let Some(render_engine) = &mut self.render_engine {
                render_engine.set_vsync(self.enable_vsync);
            }

            // Initialize GPU resources for current simulation
            if let Some(render_engine) = &mut self.render_engine {
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
            WindowEvent::CursorMoved { position, .. } => {
                // Track mouse position for picking
                self.mouse_position = (position.x as f32, position.y as f32);
            }
            WindowEvent::MouseInput { 
                button: winit::event::MouseButton::Left,
                state: winit::event::ElementState::Pressed,
                ..
            } => {
                // Handle left mouse click for object picking
                self.handle_mouse_click();
            }
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                let Some(render_engine) = self.render_engine.as_mut() else {
                    return;
                };

                // Custom frame timing that accounts for framerate limiting
                let actual_frame_time = self.last_performance_frame_time.elapsed();
                self.last_performance_frame_time = std::time::Instant::now();
                
                // Manually add frame time to performance monitor to show correct limited FPS
                self.performance_monitor.add_manual_frame_time(actual_frame_time);

                // Calculate actual delta time for simulation
                let delta_time = 1.0 / 120.0; // Fixed timestep for stability

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

                // Update instanced grid based on current simulation
                if self.simulation_manager.current_simulation_name()
                    .map(|name| name.contains("Conway"))
                    .unwrap_or(false) 
                {
                    // Conway 3D simulations - get their instanced grid data
                    if let Some(conway_data) = self.simulation_manager.get_instanced_grid_data() {
                        render_engine.update_instanced_grid_data(&conway_data);
                    } else {
                        // Conway simulation exists but no data yet
                        render_engine.update_instanced_grid_data(&Vec::new());
                    }
                } else {
                    // Non-Conway simulation - no instanced cubes for LBM
                    render_engine.update_instanced_grid_data(&Vec::new());
                }

                // Update gizmos
                self.gizmo_manager.update(
                    delta_time,
                    &mut self.scene,
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
                        // When user provides a UI callback, they have full control over UI
                        // The user can call default_transform_panel() if they want it
                        
                        // Render simulation UI first
                        self.simulation_manager.render_ui(ui, &mut self.scene);

                        // Render visualization UI (right side)
                        self.visualization_manager.render_ui(ui);

                        // Render gizmo UI
                        self.gizmo_manager.render_ui(ui, &mut self.scene);

                        // Render performance metrics if enabled
                        if self.show_performance_panel {
                            self.performance_monitor.render_ui(ui);
                        }

                        // Then render user UI callback if provided
                        ui_callback(ui, &mut self.scene, &mut self.selected_object_index);
                    });

                    // Store UI input state for object picking
                    self.ui_wants_input = ui_wants_input;

                    // Camera controls are disabled when UI has focus
                    if !ui_wants_input {
                        // Camera input processing would happen here
                    }
                } else if let Some(ui_manager) = self.ui_manager.as_mut() {
                    // If no user UI callback, still render default UI, simulation UI and visualizations
                    let ui_wants_input = ui_manager.update_logic(window, |ui| {
                        // Render default object transformation UI (left side) if enabled
                        if self.show_transform_panel {
                            default_transform_panel(
                                ui,
                                &mut self.scene,
                                &mut self.selected_object_index,
                            );
                        }

                        self.simulation_manager.render_ui(ui, &mut self.scene);
                        self.visualization_manager.render_ui(ui);
                        self.gizmo_manager.render_ui(ui, &mut self.scene);

                        // Render performance metrics if enabled
                        if self.show_performance_panel {
                            self.performance_monitor.render_ui(ui);
                        }
                    });

                    // Store UI input state for object picking
                    self.ui_wants_input = ui_wants_input;
                }

                // Apply UI transform changes to GPU buffers only when dirty
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
                let mut visualization_planes =
                    self.visualization_manager.get_visualization_planes();
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
                    render_engine
                        .render_frame_with_visualizations(&self.scene, &visualization_planes);
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
    /// This method manages framerate limiting and requests redraws at the appropriate time.
    /// It ensures continuous rendering while respecting framerate limits for performance.
    ///
    /// # Arguments
    ///
    /// * `_event_loop` - The active event loop (unused)
    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(ref window) = self.window {
            // Apply framerate limiting here to control redraw frequency
            if let Some(fps_limit) = self.framerate_limit {
                let target_frame_time = std::time::Duration::from_secs_f32(1.0 / fps_limit);
                let elapsed = self.last_frame_time.elapsed();
                
                if elapsed >= target_frame_time {
                    // Enough time has passed, request redraw
                    self.last_frame_time = std::time::Instant::now();
                    window.request_redraw();
                } else {
                    // Not enough time has passed, just short sleep
                    // The simulation runs continuously in its own update loop, 
                    // we don't need to drive it from the framerate limiter
                    std::thread::sleep(std::time::Duration::from_millis(1));
                }
            } else {
                // No framerate limit, request redraw immediately
                window.request_redraw();
            }
        }
    }
}

impl AppState {
    /// Handle mouse click for object picking
    fn handle_mouse_click(&mut self) {
        // Only pick objects if UI is not capturing input and we have a render engine
        let Some(render_engine) = self.render_engine.as_ref() else {
            return;
        };

        // Check if UI wants input (to avoid picking while interacting with UI)
        if self.ui_wants_input {
            return; // UI is capturing input, don't pick objects
        }

        // Get screen size
        let (screen_width, screen_height) = render_engine.get_surface_size();
        let screen_size = (screen_width as f32, screen_height as f32);

        // Get camera
        let camera = &self.scene.camera_manager.camera;

        // Perform object picking
        if let Some(pick_result) = self.object_picker.pick_object(
            self.mouse_position,
            screen_size,
            camera,
            &self.scene,
        ) {
            #[cfg(debug_assertions)]
            {
                println!(
                    "Picked object {} at distance {:.2}",
                    pick_result.object_index, pick_result.distance
                );
                if let Some(object) = self.scene.objects.get(pick_result.object_index) {
                    println!("Selected object: '{}'", object.name);
                }
            }

            // Update selected object index
            self.selected_object_index = Some(pick_result.object_index);
        } else {
            #[cfg(debug_assertions)]
            println!("No object picked");
            // Optionally deselect when clicking empty space
            // self.selected_object_index = None;
        }
    }
}
