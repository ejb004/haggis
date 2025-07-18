//! # Hybrid CPU/GPU Simulation Example
//!
//! This example demonstrates how to use the mid-level API to create hybrid simulations
//! that intelligently switch between CPU and GPU execution based on workload.
//!
//! ## Features Demonstrated
//! - Intelligent CPU/GPU switching
//! - Performance comparison between execution modes
//! - Resource management for both CPU and GPU
//! - Mixed simulation types running concurrently
//! - Real-time performance monitoring
//!
//! ## Usage
//! ```bash
//! cargo run --example hybrid_cpu_gpu
//! ```

use haggis::simulation::high_level::{ParticleSystem, ParticleSimulation};
use haggis::simulation::mid_level::ManagedSimulation;
use haggis::simulation::traits::Simulation;
use haggis::ui::default_transform_panel;
use std::time::Instant;

/// Hybrid simulation that can switch between CPU and GPU execution
struct HybridSimulation {
    cpu_simulation: ParticleSimulation,
    gpu_simulation: ParticleSimulation,
    use_gpu: bool,
    auto_switch: bool,
    particle_count: usize,
    gpu_threshold: usize,
    last_performance_check: Instant,
    cpu_frame_times: Vec<f32>,
    gpu_frame_times: Vec<f32>,
    performance_samples: usize,
}

impl HybridSimulation {
    fn new(initial_count: usize) -> Self {
        // Create CPU-optimized particle system
        let cpu_system = ParticleSystem::new()
            .with_count(initial_count)
            .use_cpu()  // Force CPU execution
            .with_gravity([0.0, 0.0, -9.8])
            .with_damping(0.95)
            .with_lifetime(20.0)
            .build();

        let cpu_sim = ParticleSimulation::new("CPU Particles".to_string(), cpu_system);

        // Create GPU-optimized particle system
        let gpu_system = ParticleSystem::new()
            .with_count(initial_count)
            .use_gpu()  // Force GPU execution
            .with_gravity([0.0, 0.0, -9.8])
            .with_damping(0.95)
            .with_lifetime(20.0)
            .build();

        let gpu_sim = ParticleSimulation::new("GPU Particles".to_string(), gpu_system);

        Self {
            cpu_simulation: cpu_sim,
            gpu_simulation: gpu_sim,
            use_gpu: false,
            auto_switch: true,
            particle_count: initial_count,
            gpu_threshold: 1000,  // Switch to GPU when > 1000 particles
            last_performance_check: Instant::now(),
            cpu_frame_times: Vec::new(),
            gpu_frame_times: Vec::new(),
            performance_samples: 60,  // Sample 60 frames for average
        }
    }

    fn update_particle_count(&mut self, new_count: usize) {
        self.particle_count = new_count;
        
        // This would require implementing a resize method on ParticleSystem
        // For now, we simulate the concept
        
        // Auto-switch based on particle count
        if self.auto_switch {
            let should_use_gpu = new_count > self.gpu_threshold;
            if should_use_gpu != self.use_gpu {
                self.use_gpu = should_use_gpu;
                self.cpu_frame_times.clear();
                self.gpu_frame_times.clear();
            }
        }
    }

    fn record_performance(&mut self, frame_time: f32) {
        if self.use_gpu {
            self.gpu_frame_times.push(frame_time);
            if self.gpu_frame_times.len() > self.performance_samples {
                self.gpu_frame_times.remove(0);
            }
        } else {
            self.cpu_frame_times.push(frame_time);
            if self.cpu_frame_times.len() > self.performance_samples {
                self.cpu_frame_times.remove(0);
            }
        }
    }

    fn get_average_frame_time(&self, use_gpu: bool) -> f32 {
        let times = if use_gpu { &self.gpu_frame_times } else { &self.cpu_frame_times };
        if times.is_empty() {
            0.0
        } else {
            times.iter().sum::<f32>() / times.len() as f32
        }
    }

    fn get_active_simulation_mut(&mut self) -> &mut ParticleSimulation {
        if self.use_gpu {
            &mut self.gpu_simulation
        } else {
            &mut self.cpu_simulation
        }
    }
}

impl Simulation for HybridSimulation {
    fn initialize(&mut self, scene: &mut haggis::gfx::scene::Scene) {
        self.cpu_simulation.initialize(scene);
        self.gpu_simulation.initialize(scene);
    }

    fn update(&mut self, delta_time: f32, scene: &mut haggis::gfx::scene::Scene) {
        let start_time = Instant::now();
        
        // Update the active simulation
        self.get_active_simulation_mut().update(delta_time, scene);
        
        // Record performance
        let frame_time = start_time.elapsed().as_secs_f32();
        self.record_performance(frame_time);
        
        // Performance-based auto-switching (every 2 seconds)
        if self.auto_switch && self.last_performance_check.elapsed().as_secs() >= 2 {
            self.last_performance_check = Instant::now();
            
            let cpu_avg = self.get_average_frame_time(false);
            let gpu_avg = self.get_average_frame_time(true);
            
            // Switch to GPU if it's significantly faster (>20% improvement)
            if cpu_avg > 0.0 && gpu_avg > 0.0 {
                if cpu_avg > gpu_avg * 1.2 && !self.use_gpu {
                    self.use_gpu = true;
                } else if gpu_avg > cpu_avg * 1.2 && self.use_gpu {
                    self.use_gpu = false;
                }
            }
        }
    }

    fn render_ui(&mut self, ui: &imgui::Ui) {
        // Hybrid control panel
        ui.window("Hybrid CPU/GPU Control")
            .size([400.0, 350.0], imgui::Condition::FirstUseEver)
            .build(|| {
                ui.text("Mid-Level API: Hybrid Execution");
                ui.separator();
                
                // Execution mode controls
                ui.text("Execution Mode:");
                if ui.button("CPU") {
                    self.use_gpu = false;
                    self.auto_switch = false;
                }
                ui.same_line();
                if ui.button("GPU") {
                    self.use_gpu = true;
                    self.auto_switch = false;
                }
                ui.same_line();
                if ui.button("Auto") {
                    self.auto_switch = true;
                }
                ui.spacing();
                
                // Particle count control
                ui.text("Particle Count:");
                let mut count = self.particle_count as i32;
                if ui.slider("##particles", 100, 5000, &mut count) {
                    self.update_particle_count(count as usize);
                }
                ui.spacing();
                
                // GPU threshold
                ui.text("GPU Threshold:");
                let mut threshold = self.gpu_threshold as i32;
                if ui.slider("##threshold", 500, 3000, &mut threshold) {
                    self.gpu_threshold = threshold as usize;
                }
                ui.spacing();
                
                // Current status
                ui.text("Current Status:");
                ui.text(&format!("Active: {}", if self.use_gpu { "GPU" } else { "CPU" }));
                ui.text(&format!("Particles: {}", self.particle_count));
                ui.text(&format!("Auto-Switch: {}", if self.auto_switch { "ON" } else { "OFF" }));
                ui.text(&format!("Threshold: {}", self.gpu_threshold));
                
                ui.spacing();
                if ui.button("Clear Performance Data") {
                    self.cpu_frame_times.clear();
                    self.gpu_frame_times.clear();
                }
            });

        // Performance comparison
        ui.window("Performance Comparison")
            .size([350.0, 300.0], imgui::Condition::FirstUseEver)
            .build(|| {
                ui.text("Execution Performance:");
                ui.separator();
                
                let cpu_avg = self.get_average_frame_time(false);
                let gpu_avg = self.get_average_frame_time(true);
                
                ui.text(&format!("CPU Avg Frame Time: {:.3}ms", cpu_avg * 1000.0));
                ui.text(&format!("CPU Samples: {}", self.cpu_frame_times.len()));
                if cpu_avg > 0.0 {
                    ui.text(&format!("CPU FPS: {:.1}", 1.0 / cpu_avg));
                }
                ui.spacing();
                
                ui.text(&format!("GPU Avg Frame Time: {:.3}ms", gpu_avg * 1000.0));
                ui.text(&format!("GPU Samples: {}", self.gpu_frame_times.len()));
                if gpu_avg > 0.0 {
                    ui.text(&format!("GPU FPS: {:.1}", 1.0 / gpu_avg));
                }
                ui.spacing();
                
                // Performance recommendation
                ui.separator();
                ui.text("Recommendation:");
                if cpu_avg > 0.0 && gpu_avg > 0.0 {
                    if cpu_avg < gpu_avg * 0.8 {
                        ui.text("CPU is significantly faster");
                    } else if gpu_avg < cpu_avg * 0.8 {
                        ui.text("GPU is significantly faster");
                    } else {
                        ui.text("Performance is similar");
                    }
                } else {
                    ui.text("Collecting performance data...");
                }
                ui.spacing();
                
                ui.text("Switching Logic:");
                ui.text("• Count-based: > 1000 particles → GPU");
                ui.text("• Performance-based: >20% improvement");
                ui.text("• Manual override available");
            });

        // Resource usage
        ui.window("Resource Management")
            .size([300.0, 200.0], imgui::Condition::FirstUseEver)
            .build(|| {
                ui.text("Mid-Level Resource Control:");
                ui.separator();
                
                ui.text("CPU Resources:");
                ui.text("• Thread pool utilization");
                ui.text("• Memory: Stack allocation");
                ui.text("• Cache: Optimized access patterns");
                ui.spacing();
                
                ui.text("GPU Resources:");
                ui.text("• Compute shader dispatch");
                ui.text("• Memory: GPU buffer allocation");
                ui.text("• Bandwidth: Optimized transfers");
                ui.spacing();
                
                ui.text("Hybrid Benefits:");
                ui.text("✓ Adaptive performance");
                ui.text("✓ Resource optimization");
                ui.text("✓ Workload-appropriate execution");
            });

        // Note: We can't delegate to the active simulation render_ui because it requires &mut self
        // but we're in a &mut self context. The sub-simulation UI is handled above.
    }

    fn name(&self) -> &str {
        "Hybrid CPU/GPU Simulation"
    }

    fn is_running(&self) -> bool {
        if self.use_gpu {
            self.gpu_simulation.is_running()
        } else {
            self.cpu_simulation.is_running()
        }
    }

    fn set_running(&mut self, running: bool) {
        self.cpu_simulation.set_running(running);
        self.gpu_simulation.set_running(running);
    }

    fn reset(&mut self, scene: &mut haggis::gfx::scene::Scene) {
        self.cpu_simulation.reset(scene);
        self.gpu_simulation.reset(scene);
        self.cpu_frame_times.clear();
        self.gpu_frame_times.clear();
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut haggis = haggis::default();

    // Create materials
    haggis
        .app_state
        .scene
        .add_material_rgb("hybrid_particle", 0.8, 0.4, 1.0, 0.9, 0.4);
    
    haggis
        .app_state
        .scene
        .add_material_rgb("boundary", 0.7, 0.7, 0.7, 0.4, 0.2);

    // Add visual objects
    for i in 0..50 {
        haggis
            .add_object("examples/test/cube.obj")
            .with_material("hybrid_particle")
            .with_name(&format!("hybrid_particle_{}", i))
            .with_transform([0.0, 0.0, 0.0], 0.04, 0.0);
    }

    // Add boundary visualization
    haggis
        .add_object("examples/test/ground.obj")
        .with_material("boundary")
        .with_name("boundary")
        .with_transform([0.0, 0.0, 0.0], 4.0, 0.0);

    // Create hybrid simulation
    let hybrid_sim = HybridSimulation::new(1500);  // Start with 1500 particles
    
    // Wrap with ManagedSimulation for profiling
    let managed_sim = ManagedSimulation::new(hybrid_sim)
        .with_debug(true);

    haggis.attach_simulation(managed_sim);

    // UI with hybrid controls
    haggis.set_ui(|ui, scene, selected_index| {
        default_transform_panel(ui, scene, selected_index);

        // Implementation guide
        ui.window("Hybrid Implementation Guide")
            .size([400.0, 300.0], imgui::Condition::FirstUseEver)
            .build(|| {
                ui.text("Mid-Level API: Hybrid CPU/GPU");
                ui.separator();
                
                ui.text("Implementation Steps:");
                ui.text("1. Create both CPU and GPU systems");
                ui.text("2. Implement switching logic");
                ui.text("3. Track performance metrics");
                ui.text("4. Auto-optimize based on workload");
                ui.spacing();
                
                ui.text("Switching Strategies:");
                ui.text("• Particle count thresholds");
                ui.text("• Performance-based adaptation");
                ui.text("• Manual override capability");
                ui.text("• Workload characteristics");
                ui.spacing();
                
                ui.text("Benefits:");
                ui.text("✓ Optimal performance across scales");
                ui.text("✓ Resource efficiency");
                ui.text("✓ Adaptive to hardware");
                ui.text("✓ User control when needed");
                
                ui.spacing();
                ui.text("This demonstrates the mid-level API's");
                ui.text("power for intelligent resource management.");
            });
    });

    haggis.run();
    Ok(())
}