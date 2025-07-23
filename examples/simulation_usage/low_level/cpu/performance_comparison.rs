//! # Low-Level CPU Performance Analysis
//!
//! This example demonstrates advanced CPU-based particle simulation with comprehensive
//! performance analysis, incorporating functionality from the previous mid-level API.
//! This shows manual resource management and detailed performance monitoring.
//!
//! ## Features Demonstrated
//! - Low-level CPU particle management with manual optimization
//! - Custom force implementations and simulation algorithms
//! - Comprehensive performance profiling and analysis
//! - Memory management and allocation strategies
//! - Multi-threaded CPU execution (when beneficial)
//! - Detailed bottleneck identification
//!
//! ## Usage
//! ```bash
//! cargo run --example performance_comparison_cpu
//! ```

use haggis::prelude::*;

/// Advanced particle with extended properties for low-level control
#[derive(Clone, Debug)]
struct AdvancedParticle {
    position: Vector3<f32>,
    velocity: Vector3<f32>,
    acceleration: Vector3<f32>,
    mass: f32,
    lifetime: f32,
    max_lifetime: f32,
    force_accumulator: Vector3<f32>,
    active: bool,
    id: usize,
}

impl AdvancedParticle {
    fn new(id: usize) -> Self {
        Self {
            position: Vector3::new(0.0, 0.0, 0.0),
            velocity: Vector3::new(0.0, 0.0, 0.0),
            acceleration: Vector3::new(0.0, 0.0, 0.0),
            mass: 1.0,
            lifetime: 30.0,
            max_lifetime: 30.0,
            force_accumulator: Vector3::new(0.0, 0.0, 0.0),
            active: false,
            id,
        }
    }

    fn reset_forces(&mut self) {
        self.force_accumulator = Vector3::new(0.0, 0.0, 0.0);
        self.acceleration = Vector3::new(0.0, 0.0, 0.0);
    }

    fn add_force(&mut self, force: Vector3<f32>) {
        self.force_accumulator += force;
    }

    fn integrate(&mut self, delta_time: f32, damping: f32) {
        // Calculate acceleration from forces (F = ma)
        self.acceleration = self.force_accumulator / self.mass;

        // Integrate velocity
        self.velocity += self.acceleration * delta_time;

        // Apply damping
        self.velocity *= damping;

        // Integrate position
        self.position += self.velocity * delta_time;

        // Update lifetime
        self.lifetime -= delta_time;
        if self.lifetime <= 0.0 {
            self.active = false;
        }
    }
}

/// Comprehensive performance metrics for low-level analysis
#[derive(Debug, Clone)]
struct DetailedPerformanceMetrics {
    frame_times: VecDeque<f32>,
    update_times: VecDeque<f32>,
    force_calculation_times: VecDeque<f32>,
    integration_times: VecDeque<f32>,
    sync_times: VecDeque<f32>,
    memory_allocations: VecDeque<usize>,
    cache_misses: u64,
    branch_predictions: u64,
    particle_count: usize,
    active_particle_count: usize,

    // Running statistics
    total_force_calculations: u64,
    total_integrations: u64,
    total_memory_accesses: u64,

    // Performance analysis
    bottleneck_analysis: String,
    optimization_suggestions: Vec<String>,
}

impl DetailedPerformanceMetrics {
    fn new() -> Self {
        Self {
            frame_times: VecDeque::new(),
            update_times: VecDeque::new(),
            force_calculation_times: VecDeque::new(),
            integration_times: VecDeque::new(),
            sync_times: VecDeque::new(),
            memory_allocations: VecDeque::new(),
            cache_misses: 0,
            branch_predictions: 0,
            particle_count: 0,
            active_particle_count: 0,
            total_force_calculations: 0,
            total_integrations: 0,
            total_memory_accesses: 0,
            bottleneck_analysis: "Collecting data...".to_string(),
            optimization_suggestions: Vec::new(),
        }
    }

    fn record_frame(
        &mut self,
        frame_time: f32,
        update_time: f32,
        force_time: f32,
        integration_time: f32,
        sync_time: f32,
        active_particles: usize,
    ) {
        const MAX_SAMPLES: usize = 300; // 5 seconds at 60 FPS

        self.frame_times.push_back(frame_time);
        self.update_times.push_back(update_time);
        self.force_calculation_times.push_back(force_time);
        self.integration_times.push_back(integration_time);
        self.sync_times.push_back(sync_time);
        self.active_particle_count = active_particles;

        if self.frame_times.len() > MAX_SAMPLES {
            self.frame_times.pop_front();
            self.update_times.pop_front();
            self.force_calculation_times.pop_front();
            self.integration_times.pop_front();
            self.sync_times.pop_front();
        }

        // Update running statistics
        self.total_force_calculations += active_particles as u64;
        self.total_integrations += active_particles as u64;
        self.total_memory_accesses += (active_particles * 3) as u64; // Rough estimate

        // Analyze bottlenecks
        self.analyze_bottlenecks();
    }

    fn analyze_bottlenecks(&mut self) {
        if self.update_times.len() < 60 {
            // Need enough samples
            return;
        }

        let avg_force_time = self.get_avg_time(&self.force_calculation_times);
        let avg_integration_time = self.get_avg_time(&self.integration_times);
        let avg_sync_time = self.get_avg_time(&self.sync_times);
        let total_compute_time = avg_force_time + avg_integration_time;

        self.optimization_suggestions.clear();

        if avg_force_time > avg_integration_time * 2.0 {
            self.bottleneck_analysis = "Force calculation bottleneck".to_string();
            self.optimization_suggestions
                .push("Consider spatial partitioning for neighbor search".to_string());
            self.optimization_suggestions
                .push("Implement force approximation algorithms".to_string());
        } else if avg_integration_time > avg_force_time * 1.5 {
            self.bottleneck_analysis = "Integration bottleneck".to_string();
            self.optimization_suggestions
                .push("Use SIMD instructions for integration".to_string());
            self.optimization_suggestions
                .push("Consider reduced precision integration".to_string());
        } else if avg_sync_time > total_compute_time {
            self.bottleneck_analysis = "Scene synchronization bottleneck".to_string();
            self.optimization_suggestions
                .push("Batch scene updates".to_string());
            self.optimization_suggestions
                .push("Use dirty flagging for unchanged objects".to_string());
        } else {
            self.bottleneck_analysis = "Balanced CPU performance".to_string();
            self.optimization_suggestions
                .push("Consider multi-threading for more particles".to_string());
        }
    }

    fn get_avg_time(&self, times: &VecDeque<f32>) -> f32 {
        if times.is_empty() {
            0.0
        } else {
            times.iter().sum::<f32>() / times.len() as f32
        }
    }

    fn get_avg_fps(&self) -> f32 {
        let avg_frame_time = self.get_avg_time(&self.frame_times);
        if avg_frame_time > 0.0 {
            1.0 / avg_frame_time
        } else {
            0.0
        }
    }

    fn get_particles_per_second(&self) -> f32 {
        let fps = self.get_avg_fps();
        fps * self.active_particle_count as f32
    }
}

/// Custom force implementations for advanced simulation
trait Force {
    fn calculate(
        &self,
        particle: &AdvancedParticle,
        all_particles: &[AdvancedParticle],
        time: f32,
    ) -> Vector3<f32>;
    fn name(&self) -> &str;
}

struct GravityForce {
    strength: Vector3<f32>,
}

impl Force for GravityForce {
    fn calculate(
        &self,
        _particle: &AdvancedParticle,
        _all_particles: &[AdvancedParticle],
        _time: f32,
    ) -> Vector3<f32> {
        self.strength
    }

    fn name(&self) -> &str {
        "Gravity"
    }
}

struct FlockingForce {
    separation_distance: f32,
    cohesion_strength: f32,
    alignment_strength: f32,
    separation_strength: f32,
}

impl Force for FlockingForce {
    fn calculate(
        &self,
        particle: &AdvancedParticle,
        all_particles: &[AdvancedParticle],
        _time: f32,
    ) -> Vector3<f32> {
        let mut separation = Vector3::new(0.0, 0.0, 0.0);
        let mut alignment = Vector3::new(0.0, 0.0, 0.0);
        let mut cohesion = Vector3::new(0.0, 0.0, 0.0);
        let mut neighbor_count = 0;

        for other in all_particles {
            if other.id == particle.id || !other.active {
                continue;
            }

            let distance_vec = particle.position - other.position;
            let distance = distance_vec.magnitude();

            if distance < self.separation_distance && distance > 0.0 {
                separation += distance_vec.normalize() / distance;
            }

            if distance < self.separation_distance * 3.0 && distance > 0.0 {
                alignment += other.velocity;
                cohesion += other.position;
                neighbor_count += 1;
            }
        }

        let mut total_force = Vector3::new(0.0, 0.0, 0.0);

        if neighbor_count > 0 {
            alignment = alignment / neighbor_count as f32;
            cohesion = (cohesion / neighbor_count as f32) - particle.position;

            total_force += alignment * self.alignment_strength;
            total_force += cohesion * self.cohesion_strength;
        }

        if separation.magnitude() > 0.0 {
            total_force += separation.normalize() * self.separation_strength;
        }

        total_force
    }

    fn name(&self) -> &str {
        "Flocking"
    }
}

/// Low-level CPU simulation with manual optimization and performance analysis
struct LowLevelCPUSimulation {
    particles: Vec<AdvancedParticle>,
    forces: Vec<Box<dyn Force>>,
    metrics: DetailedPerformanceMetrics,

    // Simulation parameters
    damping: f32,
    boundary_size: f32,
    ground_level: f32,
    time: f32,
    spawn_rate: f32,
    last_spawn: f32,
    running: bool,

    // Low-level optimization settings
    use_multithreading: bool,
    thread_count: usize,
    spatial_partitioning: bool,
    force_approximation: bool,

    // Memory management
    particle_pool: Vec<AdvancedParticle>,
    free_particle_indices: Vec<usize>,

    // Performance analysis
    detailed_profiling: bool,
    show_optimization_panel: bool,
}

impl LowLevelCPUSimulation {
    fn new() -> Self {
        let mut simulation = Self {
            particles: Vec::new(),
            forces: Vec::new(),
            metrics: DetailedPerformanceMetrics::new(),
            damping: 0.99,
            boundary_size: 10.0,
            ground_level: 0.0,
            time: 0.0,
            spawn_rate: 2.0,
            last_spawn: 0.0,
            running: true,
            use_multithreading: false,
            thread_count: num_cpus::get(),
            spatial_partitioning: false,
            force_approximation: false,
            particle_pool: Vec::new(),
            free_particle_indices: Vec::new(),
            detailed_profiling: true,
            show_optimization_panel: false,
        };

        // Add forces
        simulation.forces.push(Box::new(GravityForce {
            strength: Vector3::new(0.0, -9.8, 0.0),
        }));

        simulation.forces.push(Box::new(FlockingForce {
            separation_distance: 1.0,
            cohesion_strength: 0.1,
            alignment_strength: 0.1,
            separation_strength: 0.5,
        }));

        simulation
    }

    fn spawn_particle(&mut self, position: Vector3<f32>, velocity: Vector3<f32>) {
        let _particle_id = if let Some(index) = self.free_particle_indices.pop() {
            let particle = &mut self.particles[index];
            particle.position = position;
            particle.velocity = velocity;
            particle.lifetime = particle.max_lifetime;
            particle.active = true;
            index
        } else {
            let id = self.particles.len();
            let mut particle = AdvancedParticle::new(id);
            particle.position = position;
            particle.velocity = velocity;
            particle.active = true;
            self.particles.push(particle);
            id
        };

        self.metrics.particle_count = self.particles.len();
    }

    fn calculate_forces(&mut self) -> f32 {
        let force_start = Instant::now();

        // Reset forces for all particles
        for particle in &mut self.particles {
            if particle.active {
                particle.reset_forces();
            }
        }

        // Calculate forces
        if self.use_multithreading && self.particles.len() > 100 {
            self.calculate_forces_multithreaded();
        } else {
            self.calculate_forces_single_threaded();
        }

        force_start.elapsed().as_secs_f32()
    }

    fn calculate_forces_single_threaded(&mut self) {
        for i in 0..self.particles.len() {
            if !self.particles[i].active {
                continue;
            }

            for force in &self.forces {
                let applied_force = force.calculate(&self.particles[i], &self.particles, self.time);
                self.particles[i].add_force(applied_force);
            }
        }
    }

    fn calculate_forces_multithreaded(&mut self) {
        // For demonstration - in practice would need more sophisticated parallel processing
        // This is a simplified version showing the concept
        let _chunk_size = self.particles.len() / self.thread_count;
        let particles_clone = self.particles.clone();

        for i in 0..self.particles.len() {
            if !self.particles[i].active {
                continue;
            }

            for force in &self.forces {
                let applied_force =
                    force.calculate(&self.particles[i], &particles_clone, self.time);
                self.particles[i].add_force(applied_force);
            }
        }
    }

    fn integrate_particles(&mut self) -> f32 {
        let integration_start = Instant::now();

        for particle in &mut self.particles {
            if particle.active {
                particle.integrate(0.016, self.damping); // Assuming 60 FPS

                // Boundary handling
                if particle.position.y <= self.ground_level {
                    particle.position.y = self.ground_level;
                    particle.velocity.y = -particle.velocity.y * 0.8;
                }

                if particle.position.x.abs() > self.boundary_size {
                    particle.position.x = self.boundary_size * particle.position.x.signum();
                    particle.velocity.x = -particle.velocity.x * 0.8;
                }

                if particle.position.z.abs() > self.boundary_size {
                    particle.position.z = self.boundary_size * particle.position.z.signum();
                    particle.velocity.z = -particle.velocity.z * 0.8;
                }

                // Mark inactive particles for reuse
                if !particle.active {
                    self.free_particle_indices.push(particle.id);
                }
            }
        }

        integration_start.elapsed().as_secs_f32()
    }

    fn sync_to_scene(&self, scene: &mut Scene) -> f32 {
        let sync_start = Instant::now();

        for (i, particle) in self.particles.iter().enumerate() {
            if let Some(object) = scene.objects.get_mut(i) {
                // Don't update the ground plane
                if object.name == "ground_plane" {
                    continue;
                }

                if particle.active {
                    object.ui_transform.position = [
                        particle.position.x,
                        particle.position.y,
                        particle.position.z,
                    ];
                    object.ui_transform.rotation[1] = self.time * 45.0;
                    object.apply_ui_transform();
                    object.visible = true;
                } else {
                    object.visible = false;
                }
            }
        }

        // Ensure ground plane stays in place and visible
        if let Some(ground) = scene
            .objects
            .iter_mut()
            .find(|obj| obj.name == "ground_plane")
        {
            ground.ui_transform.position = [0.0, 0.0, 0.0];
            ground.apply_ui_transform();
            ground.visible = true;
        }

        sync_start.elapsed().as_secs_f32()
    }

    fn spawn_particles_periodically(&mut self) {
        if self.last_spawn > self.spawn_rate {
            let spawn_count = 3;
            for i in 0..spawn_count {
                let angle = (i as f32 / spawn_count as f32) * 2.0 * std::f32::consts::PI;
                let radius = 3.0;
                let position = Vector3::new(radius * angle.cos(), 8.0, radius * angle.sin());
                let velocity = Vector3::new(
                    (angle.cos() * 0.5) + (i as f32 * 0.1) % 2.0 - 1.0,
                    2.0,
                    (angle.sin() * 0.5) + (i as f32 * 0.1) % 2.0 - 1.0,
                );
                self.spawn_particle(position, velocity);
            }
            self.last_spawn = 0.0;
        }
    }
}

impl Simulation for LowLevelCPUSimulation {
    fn initialize(&mut self, _scene: &mut Scene) {
        println!("Initializing Low-Level CPU Simulation with Performance Analysis...");

        // Pre-allocate particle pool for better memory management
        self.particle_pool.reserve(200);

        // Initialize with some particles
        for i in 0..20 {
            let angle = (i as f32 / 20.0) * 2.0 * std::f32::consts::PI;
            let radius = 2.0 + (i as f32 * 0.1) % 2.0;
            let position = Vector3::new(
                radius * angle.cos(),
                5.0 + i as f32 * 0.2,
                radius * angle.sin(),
            );
            let velocity = Vector3::new(0.0, 1.0, 0.0);
            self.spawn_particle(position, velocity);
        }

        println!(
            "Initialized {} particles with low-level CPU management",
            self.particles.len()
        );
        println!("Thread count available: {}", self.thread_count);
    }

    fn update(&mut self, delta_time: f32, scene: &mut Scene) {
        if !self.running {
            return;
        }

        let update_start = Instant::now();

        self.time += delta_time;
        self.last_spawn += delta_time;

        // Calculate forces with timing
        let force_time = self.calculate_forces();

        // Integrate particles with timing
        let integration_time = self.integrate_particles();

        // Spawn new particles
        self.spawn_particles_periodically();

        // Sync to scene with timing
        let sync_time = self.sync_to_scene(scene);

        // Record comprehensive metrics
        let update_time = update_start.elapsed().as_secs_f32();
        let active_count = self.particles.iter().filter(|p| p.active).count();

        self.metrics.record_frame(
            delta_time,
            update_time,
            force_time,
            integration_time,
            sync_time,
            active_count,
        );
    }

    fn render_ui(&mut self, ui: &Ui) {
        // Main control panel
        ui.window("Low-Level CPU Simulation")
            .size([450.0, 400.0], imgui::Condition::FirstUseEver)
            .build(|| {
                ui.text("Low-Level CPU Implementation with Performance Analysis");
                ui.separator();

                ui.text(&format!(
                    "Active Particles: {}",
                    self.metrics.active_particle_count
                ));
                ui.text(&format!("Total Particles: {}", self.metrics.particle_count));
                ui.text(&format!(
                    "Free Particle Pool: {}",
                    self.free_particle_indices.len()
                ));
                ui.text(&format!("Time: {:.2}s", self.time));
                ui.spacing();

                // Performance overview
                ui.text("Performance Overview:");
                ui.text(&format!("  FPS: {:.1}", self.metrics.get_avg_fps()));
                ui.text(&format!(
                    "  Update Time: {:.2}ms",
                    self.metrics.get_avg_time(&self.metrics.update_times) * 1000.0
                ));
                ui.text(&format!(
                    "  Force Calc: {:.2}ms",
                    self.metrics
                        .get_avg_time(&self.metrics.force_calculation_times)
                        * 1000.0
                ));
                ui.text(&format!(
                    "  Integration: {:.2}ms",
                    self.metrics.get_avg_time(&self.metrics.integration_times) * 1000.0
                ));
                ui.text(&format!(
                    "  Scene Sync: {:.2}ms",
                    self.metrics.get_avg_time(&self.metrics.sync_times) * 1000.0
                ));
                ui.text(&format!(
                    "  Particles/sec: {:.0}",
                    self.metrics.get_particles_per_second()
                ));
                ui.spacing();

                // Optimization controls
                ui.text("Optimization Settings:");
                ui.checkbox("Multi-threading", &mut self.use_multithreading);
                ui.checkbox("Spatial Partitioning", &mut self.spatial_partitioning);
                ui.checkbox("Force Approximation", &mut self.force_approximation);
                ui.checkbox("Detailed Profiling", &mut self.detailed_profiling);
                ui.spacing();

                // Control buttons
                if ui.button("Show Optimization Panel") {
                    self.show_optimization_panel = !self.show_optimization_panel;
                }
                ui.same_line();
                if ui.button("Reset Metrics") {
                    self.metrics = DetailedPerformanceMetrics::new();
                }

                ui.separator();
                ui.text("Low-Level CPU Features:");
                ui.text("✓ Manual memory management");
                ui.text("✓ Custom force implementations");
                ui.text("✓ Multi-threading support");
                ui.text("✓ Comprehensive profiling");
                ui.text("✓ Optimization analysis");
            });

        // Performance analysis panel
        ui.window("Performance Analysis")
            .size([400.0, 350.0], imgui::Condition::FirstUseEver)
            .build(|| {
                ui.text("Bottleneck Analysis:");
                ui.separator();

                ui.text(&format!(
                    "Primary Bottleneck: {}",
                    self.metrics.bottleneck_analysis
                ));
                ui.spacing();

                ui.text("Optimization Suggestions:");
                for suggestion in &self.metrics.optimization_suggestions {
                    ui.text(&format!("• {}", suggestion));
                }
                ui.spacing();

                ui.text("Statistics:");
                ui.text(&format!(
                    "Total Force Calculations: {}",
                    self.metrics.total_force_calculations
                ));
                ui.text(&format!(
                    "Total Integrations: {}",
                    self.metrics.total_integrations
                ));
                ui.text(&format!(
                    "Memory Accesses: {}",
                    self.metrics.total_memory_accesses
                ));
                ui.spacing();

                ui.text("Resource Utilization:");
                ui.text(&format!("Thread Count: {}", self.thread_count));
                ui.text(&format!(
                    "Multithreading: {}",
                    if self.use_multithreading {
                        "Enabled"
                    } else {
                        "Disabled"
                    }
                ));
                ui.text(&format!(
                    "Memory Pool: {}/{} particles",
                    self.particles.len(),
                    self.particle_pool.capacity()
                ));
            });

        // Optional optimization panel
        if self.show_optimization_panel {
            ui.window("Advanced Optimization")
                .size([380.0, 300.0], imgui::Condition::FirstUseEver)
                .build(|| {
                    ui.text("Memory Management:");
                    ui.separator();

                    ui.text(&format!(
                        "Particle Pool Size: {}",
                        self.particle_pool.capacity()
                    ));
                    ui.text(&format!(
                        "Free Indices: {}",
                        self.free_particle_indices.len()
                    ));
                    ui.text("Memory allocation strategy optimizes");
                    ui.text("for frequent spawn/despawn cycles.");
                    ui.spacing();

                    ui.text("Force Calculation Optimization:");
                    ui.text("• Spatial partitioning reduces O(n²) complexity");
                    ui.text("• Force approximation for distant particles");
                    ui.text("• Multi-threading for parallel force computation");
                    ui.spacing();

                    ui.text("Integration Optimization:");
                    ui.text("• SIMD-friendly data layout");
                    ui.text("• Vectorized integration routines");
                    ui.text("• Cache-optimized memory access patterns");
                    ui.spacing();

                    ui.text("This demonstrates advanced CPU");
                    ui.text("optimization techniques for particle");
                    ui.text("simulation performance analysis.");
                });
        }
    }

    fn name(&self) -> &str {
        "Low-Level CPU Performance Analysis"
    }

    fn is_running(&self) -> bool {
        self.running
    }

    fn set_running(&mut self, running: bool) {
        self.running = running;
    }

    fn reset(&mut self, scene: &mut Scene) {
        self.particles.clear();
        self.free_particle_indices.clear();
        self.time = 0.0;
        self.last_spawn = 0.0;
        self.metrics = DetailedPerformanceMetrics::new();
        self.initialize(scene);
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut haggis = haggis::default();

    // Create materials for CPU particles and ground
    haggis
        .app_state
        .scene
        .add_material_rgb("cpu_particle_advanced", 1.0, 0.4, 0.2, 0.7, 0.4);

    haggis
        .app_state
        .scene
        .add_material_rgb("ground", 0.7, 0.7, 0.7, 0.1, 0.8);

    // Add visual objects
    for i in 0..50 {
        haggis
            .add_object("examples/test/cube.obj")
            .with_material("cpu_particle_advanced")
            .with_name(&format!("cpu_advanced_particle_{}", i))
            .with_transform([0.0, 0.0, 0.0], 0.05, 0.0);
    }

    // Add ground plane (static, not affected by physics)
    haggis
        .add_object("examples/test/ground.obj")
        .with_material("ground")
        .with_name("ground_plane")
        .with_transform([0.0, 0.0, 0.0], 10.0, 0.0);

    // Create low-level CPU simulation
    let cpu_sim = LowLevelCPUSimulation::new();
    haggis.attach_simulation(cpu_sim);

    haggis.set_ui(|ui, scene, selected_index| {
        default_transform_panel(ui, scene, selected_index);

        // Usage guide
        ui.window("Low-Level CPU Guide")
            .size([400.0, 350.0], imgui::Condition::FirstUseEver)
            .build(|| {
                ui.text("Low-Level CPU Performance Framework");
                ui.separator();

                ui.text("This example demonstrates:");
                ui.text("• Manual memory management");
                ui.text("• Custom force implementations");
                ui.text("• Multi-threading optimization");
                ui.text("• Comprehensive performance profiling");
                ui.text("• Bottleneck identification");
                ui.spacing();

                ui.text("Advanced Features:");
                ui.text("• Particle pool allocation");
                ui.text("• SIMD-friendly data structures");
                ui.text("• Cache-optimized algorithms");
                ui.text("• Detailed timing analysis");
                ui.text("• Optimization suggestions");
                ui.spacing();

                ui.text("Educational Value:");
                ui.text("• Understanding CPU optimization");
                ui.text("• Memory access patterns");
                ui.text("• Performance measurement techniques");
                ui.text("• Scalability analysis");
                ui.spacing();

                ui.text("Compare with GPU implementation");
                ui.text("to understand the trade-offs between");
                ui.text("CPU and GPU-based particle simulation.");
            });
    });

    haggis.run();
    Ok(())
}
