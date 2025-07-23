//! # Conway's Game of Life - CPU Implementation
//!
//! This example demonstrates Conway's Game of Life using the 2D data plane visualization system.
//! It follows the exact same pattern as cut_plane_demo but with Conway's Game of Life data.

use haggis::{simulation::BaseSimulation, CutPlane2D};
use std::time::Instant;

/// Classic Game of Life patterns
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LifePattern {
    Random,
    Glider,
    Blinker,
    GosperGun,
    Clear,
}

impl LifePattern {
    pub fn as_str(&self) -> &'static str {
        match self {
            LifePattern::Random => "Random",
            LifePattern::Glider => "Glider",
            LifePattern::Blinker => "Blinker",
            LifePattern::GosperGun => "Gosper Gun",
            LifePattern::Clear => "Clear",
        }
    }
}

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

    /// Set a specific pattern on the grid
    pub fn set_pattern(&mut self, pattern: LifePattern) {
        // Clear the grid first
        self.current_grid.fill(false);
        self.generation = 0;

        match pattern {
            LifePattern::Random => {
                use rand::Rng;
                let mut rng = rand::rng();
                for cell in self.current_grid.iter_mut() {
                    *cell = rng.random_bool(0.3);
                }
            }
            LifePattern::Glider => {
                // Place glider at center
                let center_x = self.width / 2;
                let center_y = self.height / 2;
                let glider_pattern = [(1, 0), (2, 1), (0, 2), (1, 2), (2, 2)];
                for (dx, dy) in glider_pattern.iter() {
                    let x = center_x + dx;
                    let y = center_y + dy;
                    if x < self.width && y < self.height {
                        let index = (y * self.width + x) as usize;
                        self.current_grid[index] = true;
                    }
                }
            }
            LifePattern::Blinker => {
                // Place blinker at center (vertical line)
                let center_x = self.width / 2;
                let center_y = self.height / 2;
                for dy in 0..3 {
                    let x = center_x;
                    let y = center_y + dy - 1;
                    if x < self.width && y < self.height {
                        let index = (y * self.width + x) as usize;
                        self.current_grid[index] = true;
                    }
                }
            }
            LifePattern::GosperGun => {
                // Place Gosper Glider Gun at upper left
                let gun_pattern = [
                    (24, 0),
                    (22, 1),
                    (24, 1),
                    (12, 2),
                    (13, 2),
                    (20, 2),
                    (21, 2),
                    (34, 2),
                    (35, 2),
                    (11, 3),
                    (15, 3),
                    (20, 3),
                    (21, 3),
                    (34, 3),
                    (35, 3),
                    (0, 4),
                    (1, 4),
                    (10, 4),
                    (16, 4),
                    (20, 4),
                    (21, 4),
                    (0, 5),
                    (1, 5),
                    (10, 5),
                    (14, 5),
                    (16, 5),
                    (17, 5),
                    (22, 5),
                    (24, 5),
                    (10, 6),
                    (16, 6),
                    (24, 6),
                    (11, 7),
                    (15, 7),
                    (12, 8),
                    (13, 8),
                ];
                for (x, y) in gun_pattern.iter() {
                    if *x < self.width && *y < self.height {
                        let index = (y * self.width + x) as usize;
                        self.current_grid[index] = true;
                    }
                }
            }
            LifePattern::Clear => {
                // Grid already cleared above
            }
        }
    }

    /// Get count of live cells
    pub fn live_count(&self) -> usize {
        self.current_grid.iter().filter(|&&cell| cell).count()
    }
}

/// Extended BaseSimulation with Conway's Game of Life logic
struct ConwaysCpuSimulation {
    base: BaseSimulation,
    game: GameOfLifeState,
    last_update: Instant,
    speed: f32, // Generations per second
    is_paused: bool,
    current_pattern: LifePattern,
    needs_manual_step: bool,
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
        data_plane.set_position([0.0, 2.0, 0.0].into());
        data_plane.set_size(2.0);

        // Add the visualization to the simulation
        base.add_visualization("data_plane", data_plane);

        Self {
            base,
            game,
            last_update: Instant::now(),
            speed: 3.0, // Default: 3 generations per second
            is_paused: false,
            current_pattern: LifePattern::Random,
            needs_manual_step: false,
        }
    }

    fn update_visualization(&mut self) {
        // Update the existing visualization with new game data (no recreation = no blurriness!)
        let data = self.game.to_visualization_data();
        if let Some(data_plane) = self.base.get_visualization_mut("data_plane") {
            if let Some(cut_plane) = data_plane.as_any_mut().downcast_mut::<CutPlane2D>() {
                cut_plane.update_data(data, self.game.width, self.game.height);
            }
        }
    }
}

impl haggis::simulation::traits::Simulation for ConwaysCpuSimulation {
    fn initialize(&mut self, scene: &mut haggis::gfx::scene::Scene) {
        self.base.initialize(scene);
        let live_count = self.game.live_count();
        println!(
            "🔬 Initial {} pattern: {} live cells",
            self.current_pattern.as_str(),
            live_count
        );
        println!("🚀 Conway's Game of Life CPU simulation initialized");
    }

    fn update(&mut self, delta_time: f32, scene: &mut haggis::gfx::scene::Scene) {
        // Handle manual stepping
        if self.needs_manual_step {
            self.game.step();
            self.update_visualization();
            self.needs_manual_step = false;
            self.last_update = Instant::now();
        }
        // Auto-evolve the game based on speed setting (if not paused)
        else if !self.is_paused && self.speed > 0.0 {
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
        ui.window("Conway's Game of Life CPU")
            .size([400.0, 320.0], imgui::Condition::FirstUseEver)
            .build(|| {
                ui.text("🔬 Conway's Game of Life CPU");
                ui.separator();

                ui.text(&format!("Generation: {}", self.game.generation));
                ui.text(&format!(
                    "Grid Size: {}x{}",
                    self.game.width, self.game.height
                ));
                ui.text(&format!("Live Cells: {}", self.game.live_count()));

                ui.separator();

                // Pattern selection
                ui.text("Pattern Selection:");
                let patterns = [
                    LifePattern::Glider,
                    LifePattern::Blinker,
                    LifePattern::GosperGun,
                    LifePattern::Random,
                    LifePattern::Clear,
                ];

                for &pattern in patterns.iter() {
                    if ui.radio_button_bool(pattern.as_str(), self.current_pattern == pattern) {
                        if self.current_pattern != pattern {
                            self.current_pattern = pattern;
                            self.game.set_pattern(pattern);
                            self.update_visualization();
                        }
                    }
                }

                ui.separator();

                // Controls
                if ui.button(if self.is_paused {
                    "▶ Play"
                } else {
                    "⏸ Pause"
                }) {
                    self.is_paused = !self.is_paused;
                }
                ui.same_line();

                if ui.button("⏭ Step") {
                    self.needs_manual_step = true;
                }

                ui.separator();

                // Speed control
                ui.text("Simulation Speed:");
                let mut speed_value = self.speed;
                if ui
                    .slider_config("Generations/sec", 0.1, 60.0)
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
                ui.text("Conway's Rules:");
                ui.bullet_text("Live cell: 2-3 neighbors → survives");
                ui.bullet_text("Dead cell: 3 neighbors → becomes alive");
                ui.bullet_text("Otherwise: dies or stays dead");
            });

        // Render base simulation UI
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
        self.game = GameOfLifeState::new(128, 128);
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
    println!("High-performance CPU implementation with interactive pattern selection.");
    println!();
    println!("Features:");
    println!("  • Classic Game of Life patterns (Glider, Blinker, Gosper Gun)");
    println!("  • Real-time visualization of cellular automaton (128x128)");
    println!("  • Speed control: 0.1 to 60.0 generations per second");
    println!("  • Interactive pattern selection and controls");
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
