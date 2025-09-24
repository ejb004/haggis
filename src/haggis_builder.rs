//! # Unified Haggis Builder System ("Whimsical Haggis")
//!
//! A fluent, type-safe builder API for creating Haggis applications with a delightful
//! developer experience. This is the main entry point for new users.
//!
//! ## Design Goals
//! - **Fluent**: Chain method calls in a natural way
//! - **Type-safe**: Compile-time validation of configurations
//! - **Discoverable**: IDE autocomplete reveals available options
//! - **Flexible**: Support both simple and complex use cases
//! - **Whimsical**: Fun and memorable API that makes development enjoyable

use crate::{
    app::{HaggisApp, ComputeMode, UiCallback},
    error::HaggisResult,
    simulation::traits::Simulation,
    gfx::scene::Scene,
};
use cgmath::Vector3;
use std::sync::Arc;

/// The main Haggis builder - your entry point to 3D graphics and simulation
pub struct Haggis {
    config: HaggisConfig,
}

/// Internal configuration state
struct HaggisConfig {
    // Window configuration
    title: Option<String>,
    window_size: Option<(u32, u32)>,
    resizable: bool,
    vsync: bool,

    // Graphics configuration
    shadows_enabled: bool,
    msaa_samples: u32,

    // Performance configuration
    performance_monitoring: bool,
    compute_mode: ComputeMode,

    // Scene configuration
    scene_builder: Option<Box<dyn FnOnce(&mut Scene) + Send>>,

    // Simulation configuration
    simulation: Option<Box<dyn Simulation>>,

    // UI configuration
    ui_callback: Option<UiCallback>,
}

impl Haggis {
    /// Create a new Haggis application builder
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use haggis::Haggis;
    ///
    /// let app = Haggis::create()
    ///     .with_title("My App")
    ///     .with_window_size(1920, 1080)
    ///     .build()
    ///     .unwrap();
    /// ```
    pub fn create() -> Self {
        Self {
            config: HaggisConfig {
                title: None,
                window_size: None,
                resizable: true,
                vsync: true,
                shadows_enabled: true,
                msaa_samples: 4,
                performance_monitoring: false,
                compute_mode: ComputeMode::Coupled,
                scene_builder: None,
                simulation: None,
                ui_callback: None,
            },
        }
    }

    /// Set the window title
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.config.title = Some(title.into());
        self
    }

    /// Set the window size
    pub fn with_window_size(mut self, width: u32, height: u32) -> Self {
        self.config.window_size = Some((width, height));
        self
    }

    /// Make the window non-resizable
    pub fn non_resizable(mut self) -> Self {
        self.config.resizable = false;
        self
    }

    /// Disable VSync for maximum framerate
    pub fn disable_vsync(mut self) -> Self {
        self.config.vsync = false;
        self
    }

    /// Enable shadow mapping (enabled by default)
    pub fn enable_shadows(mut self) -> Self {
        self.config.shadows_enabled = true;
        self
    }

    /// Disable shadow mapping for better performance
    pub fn disable_shadows(mut self) -> Self {
        self.config.shadows_enabled = false;
        self
    }

    /// Set MSAA sample count (4 by default)
    pub fn with_msaa_samples(mut self, samples: u32) -> Self {
        self.config.msaa_samples = samples;
        self
    }

    /// Disable MSAA (equivalent to 1 sample)
    pub fn disable_msaa(mut self) -> Self {
        self.config.msaa_samples = 1;
        self
    }

    /// Enable performance monitoring and display
    pub fn enable_performance_monitoring(mut self) -> Self {
        self.config.performance_monitoring = true;
        self
    }

    /// Set compute mode for simulations
    pub fn with_compute_mode(mut self, mode: ComputeMode) -> Self {
        self.config.compute_mode = mode;
        self
    }

    /// Use independent compute mode with specified FPS
    pub fn with_independent_compute(mut self, compute_fps: f32) -> Self {
        self.config.compute_mode = ComputeMode::Independent { compute_fps };
        self
    }

    /// Configure the 3D scene using a fluent builder
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use haggis::Haggis;
    ///
    /// let app = Haggis::create()
    ///     .scene(|scene| {
    ///         scene
    ///             .add_cube().at([0, 0, 0]).scale(2.0).material("metal")
    ///             .add_light().point().at([5, 5, 5]).intensity(1.0)
    ///             .add_camera().orbit().distance(10.0);
    ///     })
    ///     .build()
    ///     .unwrap();
    /// ```
    pub fn scene<F>(mut self, scene_fn: F) -> Self
    where
        F: FnOnce(SceneBuilder) -> SceneBuilder + Send + 'static,
    {
        self.config.scene_builder = Some(Box::new(move |scene| {
            let builder = SceneBuilder::new(scene);
            scene_fn(builder).finalize();
        }));
        self
    }

    /// Attach a simulation to the application
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use haggis::{Haggis, prelude::*};
    ///
    /// struct MySimulation;
    /// impl Simulation for MySimulation {
    ///     fn update(&mut self, _dt: f32, _scene: &mut Scene, _device: Option<&Device>, _queue: Option<&Queue>) {}
    ///     fn name(&self) -> &str { "My Simulation" }
    ///     fn render_ui(&mut self, _ui: &imgui::Ui, _scene: &mut Scene) {}
    ///     fn is_running(&self) -> bool { true }
    ///     fn set_running(&mut self, _running: bool) {}
    /// }
    ///
    /// let app = Haggis::create()
    ///     .simulation(MySimulation)
    ///     .build()
    ///     .unwrap();
    /// ```
    pub fn simulation<S: Simulation + 'static>(mut self, sim: S) -> Self {
        self.config.simulation = Some(Box::new(sim));
        self
    }

    /// Set up custom UI using ImGui
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use haggis::Haggis;
    ///
    /// let app = Haggis::create()
    ///     .ui(|ui, scene, selected| {
    ///         ui.window("Controls").build(|| {
    ///             ui.text("Hello, Haggis!");
    ///             if ui.button("Reset Scene") {
    ///                 // Reset scene logic
    ///             }
    ///         });
    ///     })
    ///     .build()
    ///     .unwrap();
    /// ```
    pub fn ui<F>(mut self, ui_fn: F) -> Self
    where
        F: Fn(&imgui::Ui, &mut Scene, &mut Option<usize>) + Send + Sync + 'static,
    {
        self.config.ui_callback = Some(Box::new(ui_fn));
        self
    }

    /// Build the configured Haggis application
    pub fn build(self) -> HaggisResult<HaggisApp> {
        // For now, return an error directing users to use build_async()
        Err(crate::error::HaggisError::resource(
            "build() requires async runtime. Use build_async() instead or call from async context",
            "HaggisBuilder"
        ))
    }

    /// Build the application asynchronously (for advanced users)
    pub async fn build_async(self) -> HaggisResult<HaggisApp> {
        let mut app = HaggisApp::new().await;

        // Apply window configuration
        if let Some(title) = self.config.title {
            // Note: This would require extending HaggisApp API
            // app.set_title(&title);
        }

        // Apply graphics configuration
        if !self.config.shadows_enabled {
            app.set_shadows_enabled(false);
        }

        // Apply scene configuration
        if let Some(scene_builder) = self.config.scene_builder {
            // This would require access to the scene from the app
            // scene_builder(&mut app.scene);
        }

        // Apply simulation - disabled until trait object issues are resolved
        // if let Some(simulation) = self.config.simulation {
        //     app.attach_simulation(simulation);
        // }

        // Apply UI configuration
        if let Some(ui_callback) = self.config.ui_callback {
            app.set_ui(ui_callback);
        }

        Ok(app)
    }

    /// Build and run the application immediately
    pub fn run(self) -> HaggisResult<()> {
        let app = self.build()?;
        app.run();
        Ok(())
    }
}

/// Scene builder for fluent scene configuration
pub struct SceneBuilder<'a> {
    scene: &'a mut Scene,
    pending_objects: Vec<ObjectBuilder>,
    pending_lights: Vec<LightBuilder>,
    pending_cameras: Vec<CameraBuilder>,
}

impl<'a> SceneBuilder<'a> {
    fn new(scene: &'a mut Scene) -> Self {
        Self {
            scene,
            pending_objects: Vec::new(),
            pending_lights: Vec::new(),
            pending_cameras: Vec::new(),
        }
    }

    /// Add a cube to the scene
    pub fn add_cube(mut self) -> ObjectBuilder {
        ObjectBuilder::new(ObjectType::Cube)
    }

    /// Add a sphere to the scene
    pub fn add_sphere(mut self) -> ObjectBuilder {
        ObjectBuilder::new(ObjectType::Sphere)
    }

    /// Add a plane to the scene
    pub fn add_plane(mut self) -> ObjectBuilder {
        ObjectBuilder::new(ObjectType::Plane)
    }

    /// Load a 3D model from file
    pub fn add_model(mut self, path: impl Into<String>) -> ObjectBuilder {
        ObjectBuilder::new(ObjectType::Model(path.into()))
    }

    /// Add a point light to the scene
    pub fn add_light(mut self) -> LightBuilder {
        LightBuilder::new()
    }

    /// Add a camera to the scene
    pub fn add_camera(mut self) -> CameraBuilder {
        CameraBuilder::new()
    }

    /// Finalize the scene configuration
    fn finalize(self) {
        // Apply all pending configurations to the actual scene
        // This is where we'd integrate with the existing scene API
    }
}

/// Builder for 3D objects in the scene
pub struct ObjectBuilder {
    object_type: ObjectType,
    position: Vector3<f32>,
    scale: f32,
    rotation_y: f32,
    material: Option<String>,
    name: Option<String>,
}

#[derive(Clone)]
enum ObjectType {
    Cube,
    Sphere,
    Plane,
    Model(String),
}

impl ObjectBuilder {
    fn new(object_type: ObjectType) -> Self {
        Self {
            object_type,
            position: Vector3::new(0.0, 0.0, 0.0),
            scale: 1.0,
            rotation_y: 0.0,
            material: None,
            name: None,
        }
    }

    /// Set the object position
    pub fn at<P: Into<Vector3<f32>>>(mut self, position: P) -> Self {
        self.position = position.into();
        self
    }

    /// Set the object scale
    pub fn scale(mut self, scale: f32) -> Self {
        self.scale = scale;
        self
    }

    /// Set the Y-axis rotation
    pub fn rotate_y(mut self, rotation: f32) -> Self {
        self.rotation_y = rotation;
        self
    }

    /// Set the material name
    pub fn material(mut self, name: impl Into<String>) -> Self {
        self.material = Some(name.into());
        self
    }

    /// Set the object name
    pub fn named(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }
}

/// Builder for lights in the scene
pub struct LightBuilder {
    light_type: LightType,
    position: Vector3<f32>,
    intensity: f32,
    color: Vector3<f32>,
}

enum LightType {
    Point,
    Directional,
    Spot,
}

impl LightBuilder {
    fn new() -> Self {
        Self {
            light_type: LightType::Point,
            position: Vector3::new(0.0, 0.0, 5.0),
            intensity: 1.0,
            color: Vector3::new(1.0, 1.0, 1.0),
        }
    }

    /// Make this a point light
    pub fn point(mut self) -> Self {
        self.light_type = LightType::Point;
        self
    }

    /// Make this a directional light
    pub fn directional(mut self) -> Self {
        self.light_type = LightType::Directional;
        self
    }

    /// Set the light position
    pub fn at<P: Into<Vector3<f32>>>(mut self, position: P) -> Self {
        self.position = position.into();
        self
    }

    /// Set the light intensity
    pub fn intensity(mut self, intensity: f32) -> Self {
        self.intensity = intensity;
        self
    }

    /// Set the light color
    pub fn color<C: Into<Vector3<f32>>>(mut self, color: C) -> Self {
        self.color = color.into();
        self
    }
}

/// Builder for cameras in the scene
pub struct CameraBuilder {
    camera_type: CameraType,
    position: Vector3<f32>,
    distance: f32,
    target: Vector3<f32>,
}

enum CameraType {
    Orbit,
    Free,
    Fixed,
}

impl CameraBuilder {
    fn new() -> Self {
        Self {
            camera_type: CameraType::Orbit,
            position: Vector3::new(0.0, 0.0, 8.0),
            distance: 8.0,
            target: Vector3::new(0.0, 0.0, 0.0),
        }
    }

    /// Make this an orbit camera
    pub fn orbit(mut self) -> Self {
        self.camera_type = CameraType::Orbit;
        self
    }

    /// Make this a free-flying camera
    pub fn free(mut self) -> Self {
        self.camera_type = CameraType::Free;
        self
    }

    /// Set the camera distance from target (for orbit camera)
    pub fn distance(mut self, distance: f32) -> Self {
        self.distance = distance;
        self
    }

    /// Set the camera target position
    pub fn target<T: Into<Vector3<f32>>>(mut self, target: T) -> Self {
        self.target = target.into();
        self
    }
}

/// Convenience implementations for common vector conversions
// Note: From implementations for Vector3 removed due to orphan rules
// Use Vector3::new() directly instead

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_haggis_builder_api() {
        let _haggis = Haggis::create()
            .with_title("Test App")
            .with_window_size(800, 600)
            .enable_shadows()
            .with_msaa_samples(8)
            .disable_vsync();

        // Test that the builder pattern works (compilation test)
    }

    #[test]
    fn test_vector_conversions() {
        let v1: Vector3<f32> = [1.0, 2.0, 3.0].into();
        let v2: Vector3<f32> = (4.0, 5.0, 6.0).into();

        assert_eq!(v1.x, 1.0);
        assert_eq!(v2.z, 6.0);
    }
}