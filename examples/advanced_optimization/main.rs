//! # Advanced Optimization Example
//!
//! This example demonstrates advanced Haggis features including:
//! - Performance monitoring and FPS tracking
//! - Dynamic object management
//! - Interactive parameter tuning
//! - Multiple object manipulation
//! - Custom simulation controls
//!
//! This shows intermediate-to-advanced usage patterns of the Haggis framework.
//!
//! ## What this example shows:
//! - Performance metrics collection and display
//! - Dynamic particle count adjustment
//! - Interactive UI controls for tuning
//! - Multiple object animation with different patterns
//! - FPS-based auto-optimization
//!
//! ## Usage:
//! ```bash
//! cargo run --example advanced_optimization
//! ```

use haggis::prelude::*;
use std::time::Instant;

/// Advanced simulation demonstrating performance monitoring and optimization
struct OptimizedSimulation {
    // Performance tracking
    frame_count: u64,
    last_fps_update: Instant,
    current_fps: f32,
    frame_times: Vec<f32>,

    // Simulation state
    particle_count: usize,
    max_particles: usize,
    auto_optimize: bool,
    target_fps: f32,

    // Animation settings
    rotation_speeds: Vec<f32>,
    orbit_radii: Vec<f32>,
    time: f32,

    // Performance optimization
    enable_complex_animation: bool,
    quality_level: usize,
}

impl OptimizedSimulation {
    fn new() -> Self {
        Self {
            frame_count: 0,
            last_fps_update: Instant::now(),
            current_fps: 0.0,
            frame_times: Vec::with_capacity(60),
            particle_count: 8,
            max_particles: 50,
            auto_optimize: true,
            target_fps: 60.0,
            rotation_speeds: vec![30.0, 45.0, 60.0, 20.0, 40.0, 35.0, 50.0, 25.0],
            orbit_radii: vec![2.0, 3.0, 4.0, 2.5, 3.5, 4.5, 5.0, 1.5],
            time: 0.0,
            enable_complex_animation: true,
            quality_level: 2,
        }
    }

    fn update_performance_metrics(&mut self, delta_time: f32) {
        self.frame_count += 1;
        self.frame_times.push(delta_time);

        // Keep only last 60 frames for moving average
        if self.frame_times.len() > 60 {
            self.frame_times.remove(0);
        }

        // Update FPS every second
        if self.last_fps_update.elapsed().as_secs_f32() >= 1.0 {
            self.current_fps = 1.0 / (self.frame_times.iter().sum::<f32>() / self.frame_times.len() as f32);
            self.last_fps_update = Instant::now();
        }

        // Auto-optimization: adjust quality based on performance
        if self.auto_optimize {
            if self.current_fps < self.target_fps * 0.9 && self.particle_count > 1 {
                // Too slow, reduce particle count or quality
                if self.quality_level > 0 {
                    self.quality_level -= 1;
                } else if self.particle_count > 1 {
                    self.particle_count = (self.particle_count - 1).max(1);
                }
            } else if self.current_fps > self.target_fps * 1.1 && self.particle_count < self.max_particles {
                // Running fast, increase particle count or quality
                if self.particle_count < self.max_particles {
                    self.particle_count += 1;
                } else if self.quality_level < 3 {
                    self.quality_level += 1;
                }
            }
        }
    }

    fn update_animations(&self, scene: &mut Scene) {
        if !self.enable_complex_animation {
            return;
        }

        // Animate visible objects with different patterns
        for (i, object) in scene.objects.iter_mut().enumerate().take(self.particle_count) {
            if i < self.rotation_speeds.len() && i < self.orbit_radii.len() {
                let speed = self.rotation_speeds[i] * (1.0 + self.quality_level as f32 * 0.2);
                let radius = self.orbit_radii[i];

                // Complex orbital motion
                let angle = self.time * speed.to_radians();
                let orbit_x = angle.cos() * radius;
                let orbit_y = angle.sin() * radius;

                object.ui_transform.position = [orbit_x, orbit_y, 2.0 + (i as f32) * 0.5];
                object.ui_transform.rotation[0] = self.time * speed * 0.5;
                object.ui_transform.rotation[1] = self.time * speed;
                object.ui_transform.rotation[2] = self.time * speed * 0.3;

                object.apply_ui_transform();
                object.visible = true;
            } else {
                object.visible = false;
            }
        }

        // Hide excess objects
        for object in scene.objects.iter_mut().skip(self.particle_count) {
            object.visible = false;
        }
    }
}

impl Simulation for OptimizedSimulation {
    fn initialize(&mut self, _scene: &mut Scene) {
        println!("Advanced optimization example initialized");
    }

    fn update(&mut self, delta_time: f32, scene: &mut Scene) {
        self.time += delta_time;
        self.update_performance_metrics(delta_time);
        self.update_animations(scene);
    }

    fn render_ui(&mut self, ui: &Ui) {
        // Performance Monitor Window
        ui.window("Performance Monitor")
            .size([350.0, 400.0], imgui::Condition::FirstUseEver)
            .position([10.0, 10.0], imgui::Condition::FirstUseEver)
            .build(|| {
                ui.text("🚀 Advanced Optimization");
                ui.separator();

                // Performance metrics
                ui.text("Performance:");
                ui.text(&format!("FPS: {:.1}", self.current_fps));
                ui.text(&format!("Frame Time: {:.2}ms",
                    self.frame_times.last().unwrap_or(&0.0) * 1000.0));
                ui.text(&format!("Frame Count: {}", self.frame_count));
                ui.spacing();

                // Simulation settings
                ui.text("Simulation:");
                ui.text(&format!("Active Objects: {}", self.particle_count));
                ui.slider("Max Objects", 1, 50, &mut self.max_particles);
                ui.checkbox("Auto Optimize", &mut self.auto_optimize);
                if !self.auto_optimize {
                    ui.slider("Object Count", 1, self.max_particles, &mut self.particle_count);
                }
                ui.slider("Target FPS", 30.0, 120.0, &mut self.target_fps);
                ui.spacing();

                // Quality settings
                ui.text("Quality:");
                ui.checkbox("Complex Animation", &mut self.enable_complex_animation);
                let quality_names = ["Low", "Medium", "High", "Ultra"];
                let mut current_quality = self.quality_level;
                if ui.combo("Quality Level", &mut current_quality, &quality_names, |item| {
                    std::borrow::Cow::Borrowed(item)
                }) {
                    self.quality_level = current_quality;
                }
                ui.spacing();

                // Performance tips
                ui.text("💡 Tips:");
                ui.bullet_text("Lower object count for better FPS");
                ui.bullet_text("Disable complex animation for speed");
                ui.bullet_text("Auto optimize adjusts quality");
            });

        // Animation Controls Window
        ui.window("Animation Controls")
            .size([300.0, 300.0], imgui::Condition::FirstUseEver)
            .position([370.0, 10.0], imgui::Condition::FirstUseEver)
            .build(|| {
                ui.text("🎮 Animation Controls");
                ui.separator();

                ui.text(&format!("Time: {:.1}s", self.time));
                ui.spacing();

                ui.text("Individual Object Speeds:");
                for (i, speed) in self.rotation_speeds.iter_mut().enumerate().take(8) {
                    ui.slider(&format!("Object {} Speed", i + 1), 0.0, 100.0, speed);
                }

                ui.spacing();
                ui.text("Orbit Radii:");
                for (i, radius) in self.orbit_radii.iter_mut().enumerate().take(8) {
                    ui.slider(&format!("Object {} Radius", i + 1), 0.5, 8.0, radius);
                }
            });

        // Statistics Window
        ui.window("Statistics")
            .size([280.0, 200.0], imgui::Condition::FirstUseEver)
            .position([10.0, 420.0], imgui::Condition::FirstUseEver)
            .build(|| {
                ui.text("📊 Performance Stats");
                ui.separator();

                if !self.frame_times.is_empty() {
                    let avg_frame_time = self.frame_times.iter().sum::<f32>() / self.frame_times.len() as f32;
                    let min_frame_time = self.frame_times.iter().fold(f32::INFINITY, |a, &b| a.min(b));
                    let max_frame_time = self.frame_times.iter().fold(0.0f32, |a, &b| a.max(b));

                    ui.text(&format!("Avg Frame Time: {:.2}ms", avg_frame_time * 1000.0));
                    ui.text(&format!("Min Frame Time: {:.2}ms", min_frame_time * 1000.0));
                    ui.text(&format!("Max Frame Time: {:.2}ms", max_frame_time * 1000.0));
                    ui.text(&format!("Frame Variance: {:.2}ms", (max_frame_time - min_frame_time) * 1000.0));
                }
                ui.spacing();

                let fps_status = if self.current_fps >= self.target_fps * 0.9 {
                    "✅ Good"
                } else if self.current_fps >= self.target_fps * 0.7 {
                    "⚠️ Fair"
                } else {
                    "❌ Poor"
                };
                ui.text(&format!("Performance: {}", fps_status));
            });
    }

    fn name(&self) -> &str {
        "Advanced Optimization"
    }

    fn is_running(&self) -> bool {
        true
    }

    fn set_running(&mut self, _running: bool) {
        // Always running
    }

    fn reset(&mut self, scene: &mut Scene) {
        *self = OptimizedSimulation::new();
        for object in scene.objects.iter_mut() {
            object.visible = true;
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

fn main() -> HaggisResult<()> {
    println!("🚀 Haggis Advanced Optimization Example");
    println!("=======================================");
    println!("Demonstrating performance monitoring and optimization techniques.");
    println!();

    // Create application with multiple objects for performance testing
    let mut app = haggis::default();

    // Create multiple objects with different materials for testing
    app.add_object("examples/test/cube.obj")
        .with_transform([0.0, 0.0, 2.0], 0.3, 0.0)
        .with_material("red_metal")
        .with_name("object_1");

    app.add_object("examples/test/sphere.obj")
        .with_transform([2.0, 0.0, 2.0], 0.3, 0.0)
        .with_material("blue_ceramic")
        .with_name("object_2");

    app.add_object("examples/test/cube.obj")
        .with_transform([0.0, 2.0, 2.0], 0.3, 0.0)
        .with_material("green_plastic")
        .with_name("object_3");

    app.add_object("examples/test/sphere.obj")
        .with_transform([-2.0, 0.0, 2.0], 0.3, 0.0)
        .with_material("gold")
        .with_name("object_4");

    app.add_object("examples/test/cube.obj")
        .with_transform([0.0, -2.0, 2.0], 0.3, 0.0)
        .with_material("silver")
        .with_name("object_5");

    app.add_object("examples/test/sphere.obj")
        .with_transform([3.0, 3.0, 3.0], 0.3, 0.0)
        .with_material("copper")
        .with_name("object_6");

    app.add_object("examples/test/cube.obj")
        .with_transform([-3.0, -3.0, 1.0], 0.3, 0.0)
        .with_material("white_plastic")
        .with_name("object_7");

    app.add_object("examples/test/sphere.obj")
        .with_transform([1.0, -1.0, 4.0], 0.3, 0.0)
        .with_material("rubber")
        .with_name("object_8");

    // Add ground plane
    app.add_object("examples/test/plane.obj")
        .with_transform([0.0, 0.0, 0.0], 15.0, 0.0)
        .with_material("ground")
        .with_name("ground_plane");

    app.attach_simulation(OptimizedSimulation::new());

    app.set_ui(|ui, scene, selected| {
        default_transform_panel(ui, scene, selected);
    });

    app.run();
    Ok(())
}