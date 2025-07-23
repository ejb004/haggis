//! # Performance Metrics System
//!
//! This module provides comprehensive performance monitoring for the Haggis engine,
//! tracking frame times, memory usage, and other key metrics that are essential
//! for simulation and rendering performance analysis.
//!
//! ## Features
//!
//! - **Frame Time Tracking**: Monitor frame times and calculate FPS
//! - **Memory Usage**: Track memory consumption and allocation patterns
//! - **Render Statistics**: GPU performance and draw call metrics
//! - **UI Integration**: Built-in ImGui panels for real-time display
//!
//! ## Usage
//!
//! ```rust
//! use haggis::performance::PerformanceMonitor;
//!
//! let mut monitor = PerformanceMonitor::new();
//! 
//! // In your main loop
//! monitor.begin_frame();
//! // ... render frame ...
//! monitor.end_frame();
//!
//! // Display metrics
//! monitor.render_ui(&ui);
//! ```

use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Comprehensive performance metrics for the engine
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    /// Current frames per second
    pub fps: f32,
    /// Average frame time in milliseconds
    pub frame_time_ms: f32,
    /// Minimum frame time in the current window
    pub min_frame_time_ms: f32,
    /// Maximum frame time in the current window
    pub max_frame_time_ms: f32,
    /// Memory usage in bytes (if available)
    pub memory_usage_bytes: Option<u64>,
    /// Number of draw calls in the last frame
    pub draw_calls: u32,
    /// Number of vertices rendered in the last frame
    pub vertex_count: u32,
    /// GPU memory usage in bytes (if available)
    pub gpu_memory_bytes: Option<u64>,
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self {
            fps: 0.0,
            frame_time_ms: 0.0,
            min_frame_time_ms: f32::MAX,
            max_frame_time_ms: 0.0,
            memory_usage_bytes: None,
            draw_calls: 0,
            vertex_count: 0,
            gpu_memory_bytes: None,
        }
    }
}

/// Performance monitoring system
pub struct PerformanceMonitor {
    /// Ring buffer of recent frame times for averaging
    frame_times: VecDeque<Duration>,
    /// Maximum number of frame times to keep for averaging
    max_samples: usize,
    /// Start time of the current frame
    frame_start: Option<Instant>,
    /// Current performance metrics
    current_metrics: PerformanceMetrics,
    /// Whether to enable detailed tracking (may impact performance)
    detailed_tracking: bool,
    /// Last time metrics were updated
    last_update: Instant,
    /// Update interval for metrics calculation
    update_interval: Duration,
}

impl PerformanceMonitor {
    /// Create a new performance monitor
    pub fn new() -> Self {
        Self {
            frame_times: VecDeque::with_capacity(120), // Store ~2 seconds at 60fps
            max_samples: 120,
            frame_start: None,
            current_metrics: PerformanceMetrics::default(),
            detailed_tracking: true,
            last_update: Instant::now(),
            update_interval: Duration::from_millis(100), // Update metrics 10 times per second
        }
    }

    /// Create a new performance monitor with custom configuration
    pub fn with_config(max_samples: usize, detailed_tracking: bool) -> Self {
        Self {
            frame_times: VecDeque::with_capacity(max_samples),
            max_samples,
            frame_start: None,
            current_metrics: PerformanceMetrics::default(),
            detailed_tracking,
            last_update: Instant::now(),
            update_interval: Duration::from_millis(100),
        }
    }

    /// Mark the beginning of a frame
    pub fn begin_frame(&mut self) {
        self.frame_start = Some(Instant::now());
    }

    /// Mark the end of a frame and update metrics
    pub fn end_frame(&mut self) {
        if let Some(start) = self.frame_start.take() {
            let frame_time = start.elapsed();
            self.add_frame_time(frame_time);
            
            // Update metrics periodically to avoid excessive computation
            if self.last_update.elapsed() >= self.update_interval {
                self.update_metrics();
                self.last_update = Instant::now();
            }
        }
    }

    /// Add a frame time sample
    fn add_frame_time(&mut self, frame_time: Duration) {
        if self.frame_times.len() >= self.max_samples {
            self.frame_times.pop_front();
        }
        self.frame_times.push_back(frame_time);
    }

    /// Update calculated metrics
    fn update_metrics(&mut self) {
        if self.frame_times.is_empty() {
            return;
        }

        // Calculate average frame time and FPS
        let total_time: Duration = self.frame_times.iter().sum();
        let avg_frame_time = total_time / self.frame_times.len() as u32;
        let avg_frame_time_ms = avg_frame_time.as_secs_f32() * 1000.0;
        
        self.current_metrics.frame_time_ms = avg_frame_time_ms;
        self.current_metrics.fps = if avg_frame_time_ms > 0.0 {
            1000.0 / avg_frame_time_ms
        } else {
            0.0
        };

        // Calculate min/max frame times
        if let (Some(min_time), Some(max_time)) = (
            self.frame_times.iter().min(),
            self.frame_times.iter().max(),
        ) {
            self.current_metrics.min_frame_time_ms = min_time.as_secs_f32() * 1000.0;
            self.current_metrics.max_frame_time_ms = max_time.as_secs_f32() * 1000.0;
        }

        // Update memory usage if detailed tracking is enabled
        if self.detailed_tracking {
            self.update_memory_usage();
        }
    }

    /// Update memory usage information (basic implementation)
    fn update_memory_usage(&mut self) {
        // Note: Accurate memory tracking in Rust is challenging without additional crates
        // This is a placeholder for basic memory information
        // In a real implementation, you might use crates like `memory-stats` or platform-specific APIs
        
        // For now, we'll leave memory tracking as optional/None
        // Users can extend this to integrate with their preferred memory tracking solution
        self.current_metrics.memory_usage_bytes = None;
        self.current_metrics.gpu_memory_bytes = None;
    }

    /// Update render statistics
    pub fn update_render_stats(&mut self, draw_calls: u32, vertex_count: u32) {
        self.current_metrics.draw_calls = draw_calls;
        self.current_metrics.vertex_count = vertex_count;
    }

    /// Get current performance metrics
    pub fn get_metrics(&self) -> &PerformanceMetrics {
        &self.current_metrics
    }

    /// Get frame time history for graphing
    pub fn get_frame_time_history(&self) -> Vec<f32> {
        self.frame_times
            .iter()
            .map(|duration| duration.as_secs_f32() * 1000.0)
            .collect()
    }

    /// Reset all metrics and history
    pub fn reset(&mut self) {
        self.frame_times.clear();
        self.current_metrics = PerformanceMetrics::default();
        self.frame_start = None;
        self.last_update = Instant::now();
    }

    /// Enable or disable detailed tracking
    pub fn set_detailed_tracking(&mut self, enabled: bool) {
        self.detailed_tracking = enabled;
    }

    /// Render performance metrics UI panel
    pub fn render_ui(&self, ui: &imgui::Ui) {
        ui.window("Performance Metrics")
            .size([300.0, 200.0], imgui::Condition::FirstUseEver)
            .position([10.0, 10.0], imgui::Condition::FirstUseEver)
            .build(|| {
                let metrics = &self.current_metrics;
                
                // FPS and frame time
                ui.text(format!("FPS: {:.1}", metrics.fps));
                ui.same_line();
                ui.text(format!("Frame Time: {:.2}ms", metrics.frame_time_ms));
                
                ui.separator();
                
                // Frame time statistics
                ui.text("Frame Time Stats:");
                ui.text(format!("  Avg: {:.2}ms", metrics.frame_time_ms));
                ui.text(format!("  Min: {:.2}ms", metrics.min_frame_time_ms));
                ui.text(format!("  Max: {:.2}ms", metrics.max_frame_time_ms));
                
                ui.separator();
                
                // Render statistics
                ui.text("Render Stats:");
                ui.text(format!("  Draw Calls: {}", metrics.draw_calls));
                ui.text(format!("  Vertices: {}", metrics.vertex_count));
                
                // Memory information (if available)
                if let Some(memory_bytes) = metrics.memory_usage_bytes {
                    ui.separator();
                    ui.text(format!("RAM: {:.1} MB", memory_bytes as f64 / 1_048_576.0));
                }
                
                if let Some(gpu_memory_bytes) = metrics.gpu_memory_bytes {
                    ui.text(format!("GPU: {:.1} MB", gpu_memory_bytes as f64 / 1_048_576.0));
                }
                
                // Frame time graph
                if !self.frame_times.is_empty() {
                    ui.separator();
                    ui.text("Frame Time History:");
                    let frame_time_history = self.get_frame_time_history();
                    ui.plot_lines("##frame_times", &frame_time_history)
                        .graph_size([260.0, 60.0])
                        .scale_min(0.0)
                        .scale_max(50.0) // 50ms max for good visibility
                        .build();
                }
            });
    }

    /// Render a compact performance overlay (minimal screen space usage)
    pub fn render_overlay(&self, ui: &imgui::Ui) {
        let display_size = ui.io().display_size;
        let metrics = &self.current_metrics;
        
        ui.window("FPS")
            .size([120.0, 60.0], imgui::Condition::Always)
            .position([display_size[0] - 130.0, 10.0], imgui::Condition::Always)
            .no_decoration()
            .no_inputs()
            .bg_alpha(0.3)
            .build(|| {
                ui.text(format!("FPS: {:.0}", metrics.fps));
                ui.text(format!("{:.1}ms", metrics.frame_time_ms));
            });
    }
}

impl Default for PerformanceMonitor {
    fn default() -> Self {
        Self::new()
    }
}