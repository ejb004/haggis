//! CPU simulation utilities and base types
//!
//! Provides helper types and utilities specifically for CPU-based simulations,
//! including common patterns and data structures that CPU simulations often need.

use super::traits::Simulation;
use crate::gfx::scene::Scene;
use imgui::Ui;

/// Base struct for CPU simulations with common functionality
///
/// Provides standard fields that most CPU simulations need, reducing boilerplate.
/// Users can either use this directly or take inspiration for their own structures.
pub struct CpuSimulationBase {
    pub name: String,
    pub running: bool,
    pub delta_accumulator: f32,
    pub step_count: u64,
}

impl CpuSimulationBase {
    /// Create a new CPU simulation base with the given name
    pub fn new(name: String) -> Self {
        Self {
            name,
            running: false,
            delta_accumulator: 0.0,
            step_count: 0,
        }
    }

    /// Record a simulation step (for debugging/statistics)
    pub fn record_step(&mut self, delta_time: f32) {
        self.step_count += 1;
        self.delta_accumulator += delta_time;
    }

    /// Get average frame time over the simulation's lifetime
    pub fn average_frame_time(&self) -> f32 {
        if self.step_count > 0 {
            self.delta_accumulator / self.step_count as f32
        } else {
            0.0
        }
    }

    /// Get simulation frequency (steps per second)
    pub fn frequency(&self) -> f32 {
        let avg = self.average_frame_time();
        if avg > 0.0 {
            1.0 / avg
        } else {
            0.0
        }
    }

    /// Reset statistics
    pub fn reset_stats(&mut self) {
        self.step_count = 0;
        self.delta_accumulator = 0.0;
    }
}
