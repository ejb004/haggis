//! Core simulation traits for the Haggis engine

use crate::gfx::scene::Scene;
use imgui::Ui;
use wgpu::{Device, Queue};

/// Core trait for user-defined simulations
pub trait Simulation {
    fn initialize(&mut self, scene: &mut Scene);
    fn update(&mut self, delta_time: f32, scene: &mut Scene);
    fn render_ui(&mut self, ui: &Ui);
    fn name(&self) -> &str;
    fn is_running(&self) -> bool;
    fn set_running(&mut self, running: bool);
    fn reset(&mut self, scene: &mut Scene);
    fn cleanup(&mut self, _scene: &mut Scene) {}

    // GPU methods with default implementations (no-op for CPU-only simulations)
    fn initialize_gpu(&mut self, _device: &Device, _queue: &Queue) {
        // Default: no GPU initialization needed
    }

    fn update_gpu(&mut self, _device: &Device, _queue: &Queue, _delta_time: f32) {
        // Default: no GPU update needed
    }

    fn apply_gpu_results_to_scene(&mut self, _device: &Device, _scene: &mut Scene) {
        // Default: no GPU results to apply
    }

    fn is_gpu_ready(&self) -> bool {
        false // Default: not GPU-ready
    }
}
