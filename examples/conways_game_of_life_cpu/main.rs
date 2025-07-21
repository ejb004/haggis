//! # Conway's Game of Life - CPU Implementation
//!
//! This example demonstrates Conway's Game of Life using the 2D data plane visualization system.
//! It follows the exact same pattern as cut_plane_demo but with Conway's Game of Life data.

use cgmath::Vector3;
use haggis::{simulation::BaseSimulation, CutPlane2D};
use std::time::Instant;

/// Simple Conway's Game of Life state
pub struct GameOfLifeState {
    width: u32,
    height: u32,
    current_grid: Vec<bool>,
    next_grid: Vec<bool>,
    running: bool,
    last_update: Instant,
    generation: u64,
}

impl GameOfLifeState {
    /// Create new Game of Life state with random initialization
    pub fn new(width: u32, height: u32) -> Self {
        let size = (width * height) as usize;
        let mut current_grid = vec![false; size];

        // Initialize with random pattern (30% alive)
        use rand::Rng;
        let mut rng = rand::rng();

        for cell in current_grid.iter_mut() {
            *cell = rng.random_bool(0.3);
        }

        Self {
            width,
            height,
            current_grid,
            next_grid: vec![false; size],
            running: false,
            last_update: Instant::now(),
            generation: 0,
        }
    }

    /// Get cell state at position (with wrapping)
    fn get_cell(&self, x: i32, y: i32) -> bool {
        let wrapped_x = ((x % self.width as i32 + self.width as i32) % self.width as i32) as u32;
        let wrapped_y = ((y % self.height as i32 + self.height as i32) % self.height as i32) as u32;
        let index = (wrapped_y * self.width + wrapped_x) as usize;

        self.current_grid.get(index).copied().unwrap_or(false)
    }

    /// Count live neighbors for a cell
    fn count_neighbors(&self, x: u32, y: u32) -> u8 {
        let mut count = 0;
        let x = x as i32;
        let y = y as i32;

        // Check all 8 neighbors
        for dy in -1..=1 {
            for dx in -1..=1 {
                if dx == 0 && dy == 0 {
                    continue; // Skip the cell itself
                }

                if self.get_cell(x + dx, y + dy) {
                    count += 1;
                }
            }
        }

        count
    }

    /// Apply Conway's rules for one generation
    pub fn step(&mut self) {
        for y in 0..self.height {
            for x in 0..self.width {
                let index = (y * self.width + x) as usize;
                let current_alive = self.current_grid[index];
                let neighbors = self.count_neighbors(x, y);

                // Conway's rules
                self.next_grid[index] = match (current_alive, neighbors) {
                    (true, 2) | (true, 3) => true, // Live cell survives
                    (false, 3) => true,            // Dead cell becomes alive
                    _ => false,                    // Cell dies or stays dead
                };
            }
        }

        // Swap grids (ping-pong)
        std::mem::swap(&mut self.current_grid, &mut self.next_grid);
        self.generation += 1;
    }

    /// Convert grid to visualization data
    pub fn to_visualization_data(&self) -> Vec<f32> {
        self.current_grid
            .iter()
            .map(|&alive| if alive { 1.0 } else { 0.0 })
            .collect()
    }
}

/// Extended BaseSimulation with Conway's Game of Life logic
struct ConwaysCpuSimulation {
    base: BaseSimulation,
    game: GameOfLifeState,
    last_update: Instant,
    speed: f32, // Generations per second
    is_paused: bool,
}

impl ConwaysCpuSimulation {
    fn new() -> Self {
        let mut base = BaseSimulation::new("Conway's Game of Life CPU");
        let game = GameOfLifeState::new(128, 128);

        // Create and configure the data plane visualization
        let mut data_plane = CutPlane2D::new();

        // Generate initial Game of Life data
        let data_2d = game.to_visualization_data();
        data_plane.update_data(data_2d, 128, 128);

        // Position the plane in 3D space - EXACT SAME as cut_plane_demo
        data_plane.set_position(cgmath::Vector3::new(0.0, 2.0, 0.0));
        data_plane.set_size(2.0);

        // Add the visualization to the simulation
        base.add_visualization("data_plane", data_plane);

        Self {
            base,
            game,
            last_update: Instant::now(),
            speed: 3.0, // Default: 3 generations per second
            is_paused: false,
        }
    }

    fn update_visualization(&mut self) {
        // Update the visualization with new game data
        let data = self.game.to_visualization_data();
        let mut data_plane = CutPlane2D::new();
        data_plane.update_data(data, self.game.width, self.game.height);
        data_plane.set_position(cgmath::Vector3::new(0.0, 2.0, 0.0));
        data_plane.set_size(2.0);

        // Remove old and add new visualization
        self.base.remove_visualization("data_plane");
        self.base.add_visualization("data_plane", data_plane);
    }
}

impl haggis::simulation::traits::Simulation for ConwaysCpuSimulation {
    fn initialize(&mut self, scene: &mut haggis::gfx::scene::Scene) {
        self.base.initialize(scene);
        println!("🚀 Conway's Game of Life simulation initialized with dynamic updating");
    }

    fn update(&mut self, delta_time: f32, scene: &mut haggis::gfx::scene::Scene) {
        // Auto-evolve the game based on speed setting (if not paused)
        if !self.is_paused && self.speed > 0.0 {
            let time_per_generation = 1.0 / self.speed;
            if self.last_update.elapsed().as_secs_f32() >= time_per_generation {
                self.game.step();
                self.update_visualization();
                self.last_update = Instant::now();
            }
        }

        // Update the base simulation (handles visualization rendering)
        self.base.update(delta_time, scene);
    }

    fn render_ui(&mut self, ui: &imgui::Ui) {
        ui.window("Conway's Game of Life")
            .size([350.0, 280.0], imgui::Condition::FirstUseEver)
            .build(|| {
                ui.text("🔬 Conway's Game of Life CPU");
                ui.separator();

                ui.text(&format!("Generation: {}", self.game.generation));
                ui.text(&format!(
                    "Grid Size: {}x{}",
                    self.game.width, self.game.height
                ));

                ui.separator();

                // Play/Pause button
                if ui.button(if self.is_paused {
                    "▶ Play"
                } else {
                    "⏸ Pause"
                }) {
                    self.is_paused = !self.is_paused;
                }
                ui.same_line();

                // Manual step button
                if ui.button("⏭ Step") {
                    self.game.step();
                    self.update_visualization();
                }
                ui.same_line();

                // Reset button
                if ui.button("🔄 Reset") {
                    self.game = GameOfLifeState::new(64, 64);
                    self.update_visualization();
                }

                ui.separator();

                // Speed control slider
                ui.text("Simulation Speed:");
                let mut speed_value = self.speed;
                if ui
                    .slider_config("Generations/sec", 0.1, 100.0)
                    .display_format("%.1f gen/sec")
                    .build(&mut speed_value)
                {
                    self.speed = speed_value;
                }

                ui.separator();

                // Status display
                ui.text("Status:");
                if self.is_paused {
                    ui.text_colored([1.0, 1.0, 0.0, 1.0], "⏸ Paused");
                } else {
                    ui.text_colored(
                        [0.0, 1.0, 0.0, 1.0],
                        &format!("▶ Running ({:.1} gen/sec)", self.speed),
                    );
                }

                ui.separator();
                ui.text("Controls:");
                ui.bullet_text("White = alive, Black = dead");
                ui.bullet_text("Use slider to control speed");
                ui.bullet_text("Pause to examine patterns");
            });

        // Render base simulation UI (visualization controls)
        self.base.render_ui(ui);
    }

    fn name(&self) -> &str {
        "Conway's Game of Life CPU"
    }

    fn is_running(&self) -> bool {
        !self.is_paused
    }

    fn set_running(&mut self, running: bool) {
        self.is_paused = !running;
    }

    fn reset(&mut self, scene: &mut haggis::gfx::scene::Scene) {
        self.game = GameOfLifeState::new(64, 64);
        self.update_visualization();
        self.base.reset(scene);
    }

    fn initialize_gpu(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        self.base.initialize_gpu(device, queue);
    }

    fn update_gpu(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, delta_time: f32) {
        self.base.update_gpu(device, queue, delta_time);
    }

    fn apply_gpu_results_to_scene(
        &mut self,
        device: &wgpu::Device,
        scene: &mut haggis::gfx::scene::Scene,
    ) {
        self.base.apply_gpu_results_to_scene(device, scene);
    }

    /// This is the key - delegate to BaseSimulation for visualization plane access
    fn as_any(&self) -> &dyn std::any::Any {
        &self.base
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔬 Conway's Game of Life - CPU Implementation");
    println!("============================================");
    println!("Dynamic Conway's Game of Life with speed control and interactive UI.");
    println!();
    println!("The 2D plane shows white cells (alive) and black cells (dead).");
    println!("Controls:");
    println!("  • Speed slider: 0.1 to 10.0 generations per second");
    println!("  • Play/Pause: Control automatic evolution");
    println!("  • Step: Advance one generation manually");
    println!("  • Reset: Generate new random starting pattern");
    println!();

    // Create the main application
    let mut app = haggis::default();

    // Create the dynamic Conway's Game of Life simulation
    let simulation = ConwaysCpuSimulation::new();

    // Attach the simulation to the app
    app.attach_simulation(simulation);

    // Add some basic 3D objects for context (same as cut_plane_demo)
    app.add_object("examples/test/cube.obj")
        .with_transform([0.0, 0.0, 0.0], 0.5, 0.0)
        .with_name("Reference Cube at Origin");

    // Add another cube for comparison at same position as plane
    app.add_object("examples/test/cube.obj")
        .with_transform([0.0, 2.0, 0.0], 0.3, 0.0)
        .with_name("Reference Cube at Plane Position");

    // Run the application
    app.run();

    Ok(())
}
