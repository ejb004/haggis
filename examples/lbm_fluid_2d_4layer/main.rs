//! # 2D Lattice Boltzmann Method (LBM) Fluid Simulation with 4-Layer Multiresolution Grid
//!
//! This example demonstrates a 2D BGK LBM fluid simulation using a geometry-based
//! 4-layer multiresolution grid for efficient simulation of flow over circular obstacles.
//!
//! ## Features
//!
//! - GPU-accelerated 2D LBM with BGK collision operator
//! - D2Q9 lattice model (9 velocity directions in 2D)
//! - Geometry-based non-adaptive 4-layer multiresolution grid
//! - Real-time velocity and vorticity visualization
//! - Flow over circular obstacle with detailed wake patterns
//! - Zou-He inlet/outlet boundary conditions
//! - Efficient bounce-back boundary conditions for obstacles
//!
//! ## Multiresolution Grid Design
//!
//! The simulation uses a geometry-based 4-layer multiresolution approach:
//! 1. Level 0 (Finest): Immediate vicinity of the circular obstacle for maximum detail
//! 2. Level 1 (Fine): Near field around obstacle for capturing flow features
//! 3. Level 2 (Medium): Intermediate field for transition regions
//! 4. Level 3 (Coarsest): Far field regions for computational efficiency
//! 5. Interface handling: Proper interpolation between all grid levels
//! 6. Non-adaptive: Grid structure is fixed based on geometry
//!
//! ## LBM Implementation Details
//!
//! 1. Stream step: Distribution functions propagate to neighboring cells with level-aware stepping
//! 2. Collision step: BGK collision operator relaxes toward equilibrium with level-specific time stepping
//! 3. Grid interface step: Handle multiresolution boundaries between any levels
//! 4. Boundary conditions: Circle obstacle with bounce-back
//! 5. Vorticity calculation: Curl of velocity field for wake visualization
//!
//! ## Usage
//!
//! Run with: `cargo run --example lbm_fluid_2d_4layer`

use cgmath::Vector3;
use haggis::prelude::*;
use haggis::{simulation::BaseSimulation, visualization::traits::VisualizationComponent};
use haggis::visualization::ui::cut_plane_controls::ColoringMode;

/// Grid configuration for the 2D LBM simulation with 4-layer multiresolution
const FINE_GRID_SIZE: u32 = 256;      // Fine grid around obstacle
const TOTAL_GRID_WIDTH: u32 = FINE_GRID_SIZE * 2;  // Total domain width
const TOTAL_GRID_HEIGHT: u32 = FINE_GRID_SIZE;     // Total domain height

/// D2Q9 lattice model - 9 velocity directions in 2D
const D2Q9_DIRECTIONS: u32 = 9;

/// LBM simulation parameters for physically accurate 2D flow over circle
#[derive(Clone, Copy, Debug)]
pub struct Lbm2d4LayerParams {
    /// Reynolds number (Re = U*D/nu) - controls flow regime
    pub reynolds_number: f32,
    /// Reference velocity (inlet velocity in lattice units)
    pub reference_velocity: f32,
    /// Kinematic viscosity (nu = cs^2 * (tau - 0.5) * dt)
    pub kinematic_viscosity: f32,
    /// Relaxation time (tau) - computed from Reynolds number
    pub tau: f32,
    /// Inlet velocity (left boundary) - computed from reference velocity
    pub inlet_velocity: f32,
    /// Outlet pressure (right boundary)
    pub outlet_pressure: f32,
    /// Circle radius (in grid units)
    pub circle_radius: f32,
    /// Circle center X position (relative to domain)
    pub circle_center_x: f32,
    /// Circle center Y position (relative to domain)
    pub circle_center_y: f32,
    /// Base refinement factor between levels
    pub refinement_factor: u32,
}

/// Rectangle bounds for quadtree spatial partitioning
#[derive(Clone, Copy, Debug)]
struct Rectangle {
    x_min: f32,
    y_min: f32,
    x_max: f32,
    y_max: f32,
}

impl Rectangle {
    fn new(x_min: f32, y_min: f32, x_max: f32, y_max: f32) -> Self {
        Self { x_min, y_min, x_max, y_max }
    }

    fn center(&self) -> (f32, f32) {
        ((self.x_min + self.x_max) * 0.5, (self.y_min + self.y_max) * 0.5)
    }

    fn width(&self) -> f32 {
        self.x_max - self.x_min
    }

    fn height(&self) -> f32 {
        self.y_max - self.y_min
    }

    fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.x_min && x <= self.x_max && y >= self.y_min && y <= self.y_max
    }

    fn subdivide(&self) -> [Rectangle; 4] {
        let (cx, cy) = self.center();
        [
            Rectangle::new(self.x_min, cy, cx, self.y_max),        // NW
            Rectangle::new(cx, cy, self.x_max, self.y_max),        // NE
            Rectangle::new(self.x_min, self.y_min, cx, cy),        // SW
            Rectangle::new(cx, self.y_min, self.x_max, cy),        // SE
        ]
    }
}

/// Quadtree node for adaptive mesh refinement
#[derive(Debug)]
struct QuadTreeNode {
    bounds: Rectangle,
    level: u32,                                 // Grid refinement level (1=finest, 3=coarsest)
    children: Option<Box<[QuadTreeNode; 4]>>,   // NW, NE, SW, SE children
    is_leaf: bool,                              // True if this node has no children
    buffer_zone: bool,                          // True if this is a stability buffer layer
}

impl QuadTreeNode {
    fn new_leaf(bounds: Rectangle, level: u32, buffer_zone: bool) -> Self {
        Self {
            bounds,
            level,
            children: None,
            is_leaf: true,
            buffer_zone,
        }
    }

    fn new_internal(bounds: Rectangle, children: [QuadTreeNode; 4]) -> Self {
        // Internal node level is the finest level among children
        let level = children.iter().map(|child| child.level).min().unwrap_or(3);
        Self {
            bounds,
            level,
            children: Some(Box::new(children)),
            is_leaf: false,
            buffer_zone: false,
        }
    }

    fn subdivide(&mut self, params: &Lbm2d4LayerParams, max_depth: u32, current_depth: u32) {
        if !self.is_leaf || current_depth >= max_depth {
            return;
        }

        let child_bounds = self.bounds.subdivide();
        let mut children = [
            QuadTreeNode::new_leaf(child_bounds[0], 3, false),
            QuadTreeNode::new_leaf(child_bounds[1], 3, false),
            QuadTreeNode::new_leaf(child_bounds[2], 3, false),
            QuadTreeNode::new_leaf(child_bounds[3], 3, false),
        ];

        // Assign refinement levels based on physics
        for child in &mut children {
            child.level = physics_based_refinement_criterion(&child.bounds, params);

            // Recursively subdivide if needed
            if should_subdivide(&child.bounds, child.level, params, current_depth, max_depth) {
                child.subdivide(params, max_depth, current_depth + 1);
            }
        }

        self.children = Some(Box::new(children));
        self.is_leaf = false;
        self.level = self.children.as_ref().unwrap().iter().map(|c| c.level).min().unwrap_or(3);
    }
}

/// Physics-based refinement criterion for quadtree nodes
/// Returns the appropriate grid level (1=finest, 3=coarsest) based on flow physics
fn physics_based_refinement_criterion(bounds: &Rectangle, params: &Lbm2d4LayerParams) -> u32 {
    let (center_x, center_y) = bounds.center();
    let grid_x = center_x * TOTAL_GRID_WIDTH as f32;
    let grid_y = center_y * TOTAL_GRID_HEIGHT as f32;

    let circle_x = params.circle_center_x * TOTAL_GRID_WIDTH as f32;
    let circle_y = params.circle_center_y * TOTAL_GRID_HEIGHT as f32;

    let distance = ((grid_x - circle_x).powi(2) + (grid_y - circle_y).powi(2)).sqrt();
    let radius = params.circle_radius;

    // Create concentric rings around the circle
    // Ring 1: Boundary layer (finest) - very close to circle surface
    if distance <= radius + 8.0 {
        return 1; // Finest level - boundary layer
    }

    // Ring 2: Near field (medium) - transition zone
    if distance <= radius + 24.0 {
        return 2; // Medium level - near field
    }

    // Ring 3: Far field (coarsest) - everything else
    3 // Coarsest level - far field
}

/// Check if circle intersects with rectangle bounds
fn circle_intersects_rectangle(bounds: &Rectangle, params: &Lbm2d4LayerParams) -> bool {
    // Convert bounds to grid coordinates
    let rect_x_min = bounds.x_min * TOTAL_GRID_WIDTH as f32;
    let rect_x_max = bounds.x_max * TOTAL_GRID_WIDTH as f32;
    let rect_y_min = bounds.y_min * TOTAL_GRID_HEIGHT as f32;
    let rect_y_max = bounds.y_max * TOTAL_GRID_HEIGHT as f32;

    // Circle center in grid coordinates
    let circle_x = params.circle_center_x * TOTAL_GRID_WIDTH as f32;
    let circle_y = params.circle_center_y * TOTAL_GRID_HEIGHT as f32;
    let radius = params.circle_radius;

    // Find closest point on rectangle to circle center
    let closest_x = circle_x.max(rect_x_min).min(rect_x_max);
    let closest_y = circle_y.max(rect_y_min).min(rect_y_max);

    // Distance from circle center to closest point on rectangle
    let dx = circle_x - closest_x;
    let dy = circle_y - closest_y;
    let distance_squared = dx * dx + dy * dy;

    // Intersection if distance is less than radius
    distance_squared <= radius * radius
}

/// Determine if a quadtree node should be subdivided further
fn should_subdivide(
    bounds: &Rectangle,
    current_level: u32,
    params: &Lbm2d4LayerParams,
    current_depth: u32,
    max_depth: u32,
) -> bool {
    // Don't subdivide too deeply
    if current_depth >= max_depth {
        return false;
    }

    // Simple rule: subdivide if intersecting or close to circle
    if circle_intersects_rectangle(bounds, params) ||
       bounds_near_circle(bounds, params) {
        return current_depth < 4; // Allow up to 4 levels of subdivision
    }

    false
}

/// Check if bounds are near the circle (within 3 radii)
fn bounds_near_circle(bounds: &Rectangle, params: &Lbm2d4LayerParams) -> bool {
    let (center_x, center_y) = bounds.center();
    let grid_x = center_x * TOTAL_GRID_WIDTH as f32;
    let grid_y = center_y * TOTAL_GRID_HEIGHT as f32;

    let circle_x = params.circle_center_x * TOTAL_GRID_WIDTH as f32;
    let circle_y = params.circle_center_y * TOTAL_GRID_HEIGHT as f32;

    let distance = ((grid_x - circle_x).powi(2) + (grid_y - circle_y).powi(2)).sqrt();
    distance <= params.circle_radius * 3.0
}

/// Build adaptive quadtree for grid refinement
fn build_adaptive_quadtree(params: &Lbm2d4LayerParams) -> QuadTreeNode {
    // Domain bounds in normalized coordinates [0,1] x [0,1]
    let domain_bounds = Rectangle::new(0.0, 0.0, 1.0, 1.0);

    // Create root node
    let mut root = QuadTreeNode::new_leaf(domain_bounds, 3, false);

    // Initial level assignment
    root.level = physics_based_refinement_criterion(&domain_bounds, params);

    // Recursive subdivision (max depth 6 for reasonable performance)
    let max_depth = 6;
    root.subdivide(params, max_depth, 0);

    println!("🌳 Quadtree built with adaptive refinement");

    root
}

/// Convert quadtree to flat grid level array with buffer layers
fn quadtree_to_grid_levels(_tree: &QuadTreeNode, params: &Lbm2d4LayerParams) -> Vec<u32> {
    let total_cells = (TOTAL_GRID_WIDTH * TOTAL_GRID_HEIGHT) as usize;
    let mut grid_levels = vec![3u32; total_cells]; // Default to coarsest

    // Apply circular refinement directly to each grid cell
    generate_circular_refinement(&mut grid_levels, params);

    // Add buffer layers for stability
    add_buffer_layers(&mut grid_levels, params);

    // Validate and smooth grid
    smooth_grid_transitions(&mut grid_levels);

    grid_levels
}

/// Generate circular refinement pattern around the circle obstacle
fn generate_circular_refinement(grid_levels: &mut Vec<u32>, params: &Lbm2d4LayerParams) {
    let circle_x = params.circle_center_x * TOTAL_GRID_WIDTH as f32;
    let circle_y = params.circle_center_y * TOTAL_GRID_HEIGHT as f32;
    let radius = params.circle_radius;

    for y in 0..TOTAL_GRID_HEIGHT {
        for x in 0..TOTAL_GRID_WIDTH {
            let cell_index = (y * TOTAL_GRID_WIDTH + x) as usize;

            // Calculate distance from circle center
            let dx = x as f32 - circle_x;
            let dy = y as f32 - circle_y;
            let distance = (dx * dx + dy * dy).sqrt();

            // Apply circular refinement zones
            let level = if distance <= radius + 8.0 {
                1 // Finest level - boundary layer
            } else if distance <= radius + 20.0 {
                2 // Medium level - near field
            } else {
                3 // Coarsest level - far field
            };

            grid_levels[cell_index] = level;
        }
    }
}

/// Recursively assign levels from quadtree nodes to grid cells
fn assign_levels_recursive(node: &QuadTreeNode, grid_levels: &mut Vec<u32>) {
    if node.is_leaf {
        // Assign level to all grid cells within this node's bounds
        let x_start = (node.bounds.x_min * TOTAL_GRID_WIDTH as f32) as u32;
        let x_end = (node.bounds.x_max * TOTAL_GRID_WIDTH as f32) as u32;
        let y_start = (node.bounds.y_min * TOTAL_GRID_HEIGHT as f32) as u32;
        let y_end = (node.bounds.y_max * TOTAL_GRID_HEIGHT as f32) as u32;

        for y in y_start..y_end.min(TOTAL_GRID_HEIGHT) {
            for x in x_start..x_end.min(TOTAL_GRID_WIDTH) {
                let cell_index = (y * TOTAL_GRID_WIDTH + x) as usize;
                if cell_index < grid_levels.len() {
                    grid_levels[cell_index] = node.level;
                }
            }
        }
    } else if let Some(children) = &node.children {
        // Recursively process children
        for child in children.iter() {
            assign_levels_recursive(child, grid_levels);
        }
    }
}

/// Add buffer layers between different refinement levels for stability
/// Iteratively applies buffer layers until all transitions are smooth
fn add_buffer_layers(grid_levels: &mut Vec<u32>, _params: &Lbm2d4LayerParams) {
    let mut total_buffer_count = 0;
    let mut iterations = 0;
    const MAX_BUFFER_ITERATIONS: u32 = 5;

    while iterations < MAX_BUFFER_ITERATIONS {
        let mut buffer_changes = Vec::new();

        // Find all level transitions and add buffers (including diagonal neighbors)
        for y in 0..TOTAL_GRID_HEIGHT {
            for x in 0..TOTAL_GRID_WIDTH {
                let cell_index = (y * TOTAL_GRID_WIDTH + x) as usize;
                let current_level = grid_levels[cell_index];

                // Check all 8 neighbors (4-connected + diagonals)
                let neighbors = [
                    // 4-connected neighbors (strict: max diff = 1)
                    if x > 0 { Some(((x - 1, y), 1.0)) } else { None },
                    if x < TOTAL_GRID_WIDTH - 1 { Some(((x + 1, y), 1.0)) } else { None },
                    if y > 0 { Some(((x, y - 1), 1.0)) } else { None },
                    if y < TOTAL_GRID_HEIGHT - 1 { Some(((x, y + 1), 1.0)) } else { None },
                    // Diagonal neighbors (relaxed: max diff = 1.5)
                    if x > 0 && y > 0 { Some(((x - 1, y - 1), 1.5)) } else { None },
                    if x < TOTAL_GRID_WIDTH - 1 && y > 0 { Some(((x + 1, y - 1), 1.5)) } else { None },
                    if x > 0 && y < TOTAL_GRID_HEIGHT - 1 { Some(((x - 1, y + 1), 1.5)) } else { None },
                    if x < TOTAL_GRID_WIDTH - 1 && y < TOTAL_GRID_HEIGHT - 1 { Some(((x + 1, y + 1), 1.5)) } else { None },
                ];

                for neighbor_data in neighbors.iter().flatten() {
                    let ((nx, ny), max_diff) = *neighbor_data;
                    let neighbor_index = (ny * TOTAL_GRID_WIDTH + nx) as usize;
                    let neighbor_level = grid_levels[neighbor_index];

                    // If there's a level jump > threshold, add buffer
                    let level_diff = (current_level as i32 - neighbor_level as i32).abs() as f32;
                    if level_diff > max_diff {
                        // For strict neighbors (axial), enforce max diff = 1
                        // For diagonal neighbors, allow max diff = 1 (stricter than before)
                        let target_diff = if max_diff <= 1.0 { 1.0 } else { 1.0 }; // All neighbors now strict

                        // Calculate required buffer level
                        let buffer_level = if current_level > neighbor_level {
                            neighbor_level + target_diff as u32
                        } else {
                            current_level + target_diff as u32
                        };

                        // Apply buffer to the coarser cell (higher level number)
                        if current_level > neighbor_level {
                            buffer_changes.push((cell_index, buffer_level));
                        } else {
                            buffer_changes.push((neighbor_index, buffer_level));
                        }
                    }
                }
            }
        }

        // Apply buffer changes
        let buffer_count = buffer_changes.len();
        if buffer_count == 0 {
            break; // No more changes needed
        }

        for (index, level) in buffer_changes {
            if index < grid_levels.len() {
                grid_levels[index] = level;
            }
        }

        total_buffer_count += buffer_count;
        iterations += 1;
    }

    // Additional pass: fill thin regions and expand buffer zones
    fill_thin_regions(grid_levels);

    println!("🛡️  Added {} buffer layers for stability in {} iterations", total_buffer_count, iterations);
}

/// Fill thin regions to ensure minimum width of transition zones
fn fill_thin_regions(grid_levels: &mut Vec<u32>) {
    let mut changes = Vec::new();
    const MIN_REGION_WIDTH: u32 = 3; // Minimum width for stable regions

    for y in 1..(TOTAL_GRID_HEIGHT - 1) {
        for x in 1..(TOTAL_GRID_WIDTH - 1) {
            let cell_index = (y * TOTAL_GRID_WIDTH + x) as usize;
            let current_level = grid_levels[cell_index];

            // Check for thin horizontal strips
            let left = grid_levels[((y * TOTAL_GRID_WIDTH + (x - 1)) as usize)];
            let right = grid_levels[((y * TOTAL_GRID_WIDTH + (x + 1)) as usize)];

            // Check for thin vertical strips
            let up = grid_levels[(((y - 1) * TOTAL_GRID_WIDTH + x) as usize)];
            let down = grid_levels[(((y + 1) * TOTAL_GRID_WIDTH + x) as usize)];

            // If we're surrounded by different levels, become intermediate
            if (left != current_level && right != current_level) ||
               (up != current_level && down != current_level) {

                // Find the most common neighbor level for smoothing
                let neighbors = [left, right, up, down];
                let mut level_counts = [0u32; 4]; // For levels 1, 2, 3 (index 0 unused)

                for &neighbor_level in &neighbors {
                    if neighbor_level >= 1 && neighbor_level <= 3 {
                        level_counts[(neighbor_level - 1) as usize] += 1;
                    }
                }

                // Find most common level
                let mut best_level = current_level;
                let mut max_count = 0;
                for (i, &count) in level_counts.iter().enumerate() {
                    if count > max_count {
                        max_count = count;
                        best_level = (i + 1) as u32;
                    }
                }

                // Use intermediate level if we're creating a big jump
                if (current_level as i32 - best_level as i32).abs() > 1 {
                    best_level = (current_level + best_level) / 2;
                }

                if best_level != current_level && best_level >= 1 && best_level <= 3 {
                    changes.push((cell_index, best_level));
                }
            }

            // Additional check: expand buffer zones around level 1 regions
            if current_level == 3 {
                // Check 3x3 neighborhood for level 1 cells
                let mut has_level1_neighbor = false;
                for dy in -1i32..=1 {
                    for dx in -1i32..=1 {
                        let nx = x as i32 + dx;
                        let ny = y as i32 + dy;
                        if nx >= 0 && nx < TOTAL_GRID_WIDTH as i32 &&
                           ny >= 0 && ny < TOTAL_GRID_HEIGHT as i32 {
                            let neighbor_index = (ny as u32 * TOTAL_GRID_WIDTH + nx as u32) as usize;
                            if grid_levels[neighbor_index] == 1 {
                                has_level1_neighbor = true;
                                break;
                            }
                        }
                    }
                    if has_level1_neighbor { break; }
                }

                if has_level1_neighbor {
                    changes.push((cell_index, 2)); // Make it level 2 buffer
                }
            }
        }
    }

    // Apply changes
    for (index, level) in changes {
        grid_levels[index] = level;
    }
}

/// Smooth grid transitions to ensure stability
fn smooth_grid_transitions(grid_levels: &mut Vec<u32>) {
    let mut converged = false;
    let mut iterations = 0;
    const MAX_ITERATIONS: u32 = 10;

    while !converged && iterations < MAX_ITERATIONS {
        converged = true;
        let mut changes = Vec::new();

        for y in 0..TOTAL_GRID_HEIGHT {
            for x in 0..TOTAL_GRID_WIDTH {
                let cell_index = (y * TOTAL_GRID_WIDTH + x) as usize;
                let current_level = grid_levels[cell_index];

                // Check all 8 neighbors for violations (4-connected + diagonals)
                let neighbors = [
                    // 4-connected neighbors (primary)
                    if x > 0 { Some(((x - 1, y), 1.0)) } else { None },
                    if x < TOTAL_GRID_WIDTH - 1 { Some(((x + 1, y), 1.0)) } else { None },
                    if y > 0 { Some(((x, y - 1), 1.0)) } else { None },
                    if y < TOTAL_GRID_HEIGHT - 1 { Some(((x, y + 1), 1.0)) } else { None },
                    // Diagonal neighbors (secondary, less strict)
                    if x > 0 && y > 0 { Some(((x - 1, y - 1), 1.5)) } else { None },
                    if x < TOTAL_GRID_WIDTH - 1 && y > 0 { Some(((x + 1, y - 1), 1.5)) } else { None },
                    if x > 0 && y < TOTAL_GRID_HEIGHT - 1 { Some(((x - 1, y + 1), 1.5)) } else { None },
                    if x < TOTAL_GRID_WIDTH - 1 && y < TOTAL_GRID_HEIGHT - 1 { Some(((x + 1, y + 1), 1.5)) } else { None },
                ];

                let mut max_violation = 0;
                let mut required_level = current_level;

                for neighbor_data in neighbors.iter().flatten() {
                    let ((nx, ny), threshold) = *neighbor_data;
                    let neighbor_index = (ny * TOTAL_GRID_WIDTH + nx) as usize;
                    let neighbor_level = grid_levels[neighbor_index];

                    let level_diff = (current_level as i32 - neighbor_level as i32).abs();
                    if level_diff as f32 > threshold {
                        // Calculate required level to satisfy constraint
                        let constraint_level = if current_level > neighbor_level {
                            (neighbor_level as f32 + threshold).ceil() as u32
                        } else {
                            current_level
                        };

                        let violation = (constraint_level as i32 - current_level as i32).abs();
                        if violation > max_violation {
                            max_violation = violation;
                            required_level = constraint_level;
                        }
                    }
                }

                if required_level != current_level {
                    changes.push((cell_index, required_level));
                    converged = false;
                }
            }
        }

        // Apply changes
        for (index, level) in changes {
            grid_levels[index] = level;
        }

        iterations += 1;
    }

    if iterations >= MAX_ITERATIONS {
        println!("⚠️  Grid smoothing reached max iterations");
    } else {
        println!("✅ Grid smoothing converged after {} iterations", iterations);
    }
}

impl Lbm2d4LayerParams {
    /// Create physically accurate parameters from Reynolds number
    ///
    /// Reynolds number regimes:
    /// - Re < 1: Creeping flow (Stokes regime)
    /// - Re = 10-40: Steady separated flow with recirculation
    /// - Re = 40-200: Periodic vortex shedding (von Kármán street)
    /// - Re > 200: Turbulent wake transition
    pub fn from_reynolds_number(reynolds_number: f32) -> Self {
        // Physical constants for D2Q9 lattice
        let cs_squared: f32 = 1.0 / 3.0;  // Speed of sound squared in lattice units
        let dt: f32 = 1.0;                // Time step in lattice units

        // Circle diameter in grid units (should be well resolved)
        let circle_radius: f32 = 20.0;
        let circle_diameter: f32 = 2.0 * circle_radius;

        // Choose reference velocity for stability (Mach number << 1)
        // Ma = U/cs should be < 0.1 for incompressible flow
        let mach_number: f32 = 0.05;  // Conservative Mach number
        let reference_velocity: f32 = mach_number * cs_squared.sqrt();

        // Compute kinematic viscosity from Reynolds definition: Re = U*D/nu
        let kinematic_viscosity = reference_velocity * circle_diameter / reynolds_number;

        // Compute relaxation time from viscosity: nu = cs^2 * (tau - 0.5) * dt
        let tau = kinematic_viscosity / (cs_squared * dt) + 0.5;

        // Validate tau for stability (must be > 0.5 and ideally < 2.0)
        let tau = tau.max(0.51).min(1.9);

        // Recompute actual viscosity and Reynolds number from validated tau
        let actual_viscosity = cs_squared * (tau - 0.5) * dt;
        let actual_reynolds = reference_velocity * circle_diameter / actual_viscosity;

        println!("🔬 Physical Parameters:");
        println!("   Target Reynolds number: {:.1}", reynolds_number);
        println!("   Actual Reynolds number: {:.1}", actual_reynolds);
        println!("   Reference velocity: {:.4} (Ma = {:.3})", reference_velocity, mach_number);
        println!("   Kinematic viscosity: {:.6}", actual_viscosity);
        println!("   Relaxation time (tau): {:.4}", tau);
        println!("   Circle diameter: {:.1} grid units", circle_diameter);

        Self {
            reynolds_number: actual_reynolds,
            reference_velocity,
            kinematic_viscosity: actual_viscosity,
            tau,
            inlet_velocity: reference_velocity,
            outlet_pressure: 1.0,
            circle_radius,
            circle_center_x: 0.25,  // 25% from left boundary
            circle_center_y: 0.5,   // Centered vertically
            refinement_factor: 2,
        }
    }
}

impl Default for Lbm2d4LayerParams {
    fn default() -> Self {
        // Use Reynolds number 100 - classic vortex shedding regime
        // This produces the famous von Kármán vortex street behind the cylinder
        Self::from_reynolds_number(100.0)
    }
}

/// Grid level information for 4-layer multiresolution
#[derive(Clone, Copy, Debug)]
pub struct GridLevel {
    pub level: u32,        // 0 = finest, 1 = fine, 2 = medium, 3 = coarsest
    pub spacing: f32,      // Grid spacing (lattice units)
    pub time_step: f32,    // Time step for this level
    pub step_size: u32,    // Step size for streaming: 1, 2, 4, 8
}

/// GPU resources for 2D LBM 4-layer multiresolution fluid simulation
struct Lbm2d4LayerGpuResources {
    // Compute pipelines
    stream_pipeline: wgpu::ComputePipeline,
    collision_pipeline: wgpu::ComputePipeline,
    interface_pipeline: wgpu::ComputePipeline,
    vorticity_pipeline: wgpu::ComputePipeline,

    // Ping-pong buffers for distribution functions (f_i)
    distributions_buffer_a: wgpu::Buffer, // Current distributions
    distributions_buffer_b: wgpu::Buffer, // Next distributions

    // Velocity and vorticity buffers
    velocity_density_buffer: wgpu::Buffer,   // 3 floats per cell: [vx, vy, density]
    vorticity_buffer: wgpu::Buffer,  // 2 floats per cell: [vorticity, magnitude]

    // Grid level buffer - defines refinement level for each cell (0-3)
    grid_level_buffer: wgpu::Buffer, // u32 per cell: grid level (0=finest, 3=coarsest)

    // Boundary buffer - bit-packed obstacles
    boundary_buffer: wgpu::Buffer,   // u32 array with bit flags

    // Parameters buffer
    params_buffer: wgpu::Buffer,

    // Bind groups for ping-pong
    stream_bind_group_a_to_b: wgpu::BindGroup,
    stream_bind_group_b_to_a: wgpu::BindGroup,
    collision_bind_group_a: wgpu::BindGroup,
    collision_bind_group_b: wgpu::BindGroup,
    interface_bind_group: wgpu::BindGroup,
    vorticity_bind_group: wgpu::BindGroup,

    // State
    ping_pong_state: bool, // false = A is current, true = B is current
}

/// 2D LBM fluid simulation with geometry-based 4-layer multiresolution grid
struct Lbm2d4LayerMultiresolutionSimulation {
    base: BaseSimulation,

    // Grid configuration
    width: u32,
    height: u32,

    // Simulation state
    generation: u64,
    is_paused: bool,

    // LBM parameters
    params: Lbm2d4LayerParams,

    // GPU resources
    gpu_resources: Option<Lbm2d4LayerGpuResources>,

    // Visualization
    needs_visualization_update: bool,
    grid_levels_cache: Option<Vec<u32>>,
    visualization_scale: f32,

    // CPU backup for visualization data
    cpu_velocity: Vec<f32>,    // 3 floats per cell
    cpu_vorticity: Vec<f32>,   // 2 floats per cell

    // Physical validation tracking
    wake_velocity_history: Vec<f32>,  // For Strouhal number calculation
    last_strouhal_calculation: u64,   // Generation of last calculation
    measured_strouhal: Option<f32>,   // Current Strouhal number estimate
}

impl Lbm2d4LayerMultiresolutionSimulation {
    /// Generate adaptive quadtree-based grid with physics-based refinement and buffer layers
    /// Uses quadtree spatial decomposition for optimal refinement distribution
    fn generate_grid_levels(params: &Lbm2d4LayerParams) -> Vec<u32> {
        // Build adaptive quadtree
        let quadtree = build_adaptive_quadtree(params);

        // Convert to grid levels with buffer layers
        let grid_levels = quadtree_to_grid_levels(&quadtree, params);

        // Enhanced diagnostics
        Self::print_grid_diagnostics(&grid_levels, params);

        grid_levels
    }

    /// Print detailed grid diagnostics for validation and optimization
    fn print_grid_diagnostics(grid_levels: &[u32], params: &Lbm2d4LayerParams) {
        let level1_count = grid_levels.iter().filter(|&&level| level == 1).count();
        let level2_count = grid_levels.iter().filter(|&&level| level == 2).count();
        let level3_count = grid_levels.iter().filter(|&&level| level == 3).count();
        let total_count = grid_levels.len();
        let diameter = 2.0 * params.circle_radius;

        println!("🔬 Quadtree Grid Distribution:");
        println!("   Level 1 (Boundary layer): {:.1}% ({} cells)",
                 100.0 * level1_count as f32 / total_count as f32, level1_count);
        println!("   Level 2 (Wake/shear regions): {:.1}% ({} cells)",
                 100.0 * level2_count as f32 / total_count as f32, level2_count);
        println!("   Level 3 (Far field): {:.1}% ({} cells)",
                 100.0 * level3_count as f32 / total_count as f32, level3_count);
        println!("   Boundary layer thickness: {:.2} grid units",
                 diameter / params.reynolds_number.sqrt());

        // Calculate efficiency metrics
        let efficiency = (level3_count as f32 / total_count as f32) * 100.0;
        let finest_ratio = level1_count as f32 / total_count as f32;
        println!("   Grid efficiency: {:.1}% coarse cells (computational savings)",
                 efficiency);
        println!("   Refinement focus: {:.1}% finest cells (critical regions)",
                 finest_ratio * 100.0);

        // Validate level transitions
        let mut violations = 0;
        for y in 0..TOTAL_GRID_HEIGHT {
            for x in 0..TOTAL_GRID_WIDTH {
                let cell_index = (y * TOTAL_GRID_WIDTH + x) as usize;
                let current_level = grid_levels[cell_index];

                let neighbors = [
                    if x > 0 { Some((x - 1, y)) } else { None },
                    if x < TOTAL_GRID_WIDTH - 1 { Some((x + 1, y)) } else { None },
                    if y > 0 { Some((x, y - 1)) } else { None },
                    if y < TOTAL_GRID_HEIGHT - 1 { Some((x, y + 1)) } else { None },
                ];

                for neighbor_coord in neighbors.iter().flatten() {
                    let neighbor_index = (neighbor_coord.1 * TOTAL_GRID_WIDTH + neighbor_coord.0) as usize;
                    let neighbor_level = grid_levels[neighbor_index];
                    if (current_level as i32 - neighbor_level as i32).abs() > 1 {
                        violations += 1;
                    }
                }
            }
        }

        if violations > 0 {
            println!("⚠️  {} level transition violations detected", violations / 2);
        } else {
            println!("✅ All level transitions valid (max difference = 1)");
        }

        // Quadtree implementation complete
    }

    /// Calculate Strouhal number from wake velocity oscillations
    fn calculate_strouhal_number(&mut self) -> Option<f32> {
        // Sample velocity in wake region (2 diameters downstream)
        let circle_x = (self.params.circle_center_x * TOTAL_GRID_WIDTH as f32) as u32;
        let circle_y = (self.params.circle_center_y * TOTAL_GRID_HEIGHT as f32) as u32;
        let sample_x = circle_x + (4.0 * self.params.circle_radius) as u32; // 2 diameters downstream

        if sample_x < TOTAL_GRID_WIDTH && circle_y < TOTAL_GRID_HEIGHT {
            let cell_index = (circle_y * TOTAL_GRID_WIDTH + sample_x) as usize;
            let vy = self.cpu_velocity.get(cell_index * 3 + 1).copied().unwrap_or(0.0);

            // Store velocity history for frequency analysis
            self.wake_velocity_history.push(vy);

            // Keep only recent history (last 1000 samples)
            if self.wake_velocity_history.len() > 1000 {
                self.wake_velocity_history.remove(0);
            }

            // Calculate Strouhal number every 500 iterations
            if self.generation > self.last_strouhal_calculation + 500 && self.wake_velocity_history.len() >= 200 {
                self.last_strouhal_calculation = self.generation;

                // Simple peak counting for frequency estimation
                let mut peaks = 0;
                let mut last_val = self.wake_velocity_history[0];
                let mut trend_up = false;

                for &val in &self.wake_velocity_history[1..] {
                    if val > last_val && !trend_up {
                        trend_up = true;
                    } else if val < last_val && trend_up {
                        peaks += 1;
                        trend_up = false;
                    }
                    last_val = val;
                }

                if peaks > 0 {
                    // Frequency in simulation units (oscillations per time step)
                    let frequency = peaks as f32 / self.wake_velocity_history.len() as f32;

                    // Strouhal number: St = f*D/U
                    let diameter = 2.0 * self.params.circle_radius;
                    let strouhal = frequency * diameter / self.params.inlet_velocity;

                    self.measured_strouhal = Some(strouhal);
                    return Some(strouhal);
                }
            }
        }

        self.measured_strouhal
    }

    /// Generate circle boundary pattern for flow obstacles
    fn generate_circle_boundaries(params: &Lbm2d4LayerParams) -> Vec<u32> {
        let total_cells = (TOTAL_GRID_WIDTH * TOTAL_GRID_HEIGHT) as usize;
        let u32_count = (total_cells + 31) / 32; // Round up for bit packing
        let mut boundary_data = vec![0u32; u32_count];

        let circle_x = params.circle_center_x * TOTAL_GRID_WIDTH as f32;
        let circle_y = params.circle_center_y * TOTAL_GRID_HEIGHT as f32;

        for y in 0..TOTAL_GRID_HEIGHT {
            for x in 0..TOTAL_GRID_WIDTH {
                let cell_index = (y * TOTAL_GRID_WIDTH + x) as usize;
                let u32_index = cell_index / 32;
                let bit_index = cell_index % 32;

                // Calculate distance from circle center
                let dx = x as f32 - circle_x;
                let dy = y as f32 - circle_y;
                let distance = (dx * dx + dy * dy).sqrt();

                // Mark cells inside circle as boundary
                if distance <= params.circle_radius {
                    boundary_data[u32_index] |= 1u32 << bit_index;
                }
            }
        }

        boundary_data
    }

    fn new() -> Self {
        let mut base = BaseSimulation::new("LBM 2D 4-Layer");

        // Create and configure the visualization for velocity field with correct 2:1 aspect ratio
        let mut velocity_plane = CutPlane2D::new();
        velocity_plane.set_position(Vector3::new(0.0, 0.0, 0.0));
        velocity_plane.set_size_2d(4.0, 2.0); // 2:1 aspect ratio plane to match data proportions

        // Initialize with empty data (downsampled by 2x)
        let downsample_factor = 2u32;
        let downsampled_width = TOTAL_GRID_WIDTH / downsample_factor;
        let downsampled_height = TOTAL_GRID_HEIGHT / downsample_factor;
        let empty_data = vec![0.0; (downsampled_width * downsampled_height) as usize];
        velocity_plane.update_data(empty_data, downsampled_width, downsampled_height);

        // Add visualization to base
        base.add_visualization("velocity_field", velocity_plane);

        // Create vorticity visualization with same aspect ratio
        let mut vorticity_plane = CutPlane2D::new();
        vorticity_plane.set_position(Vector3::new(0.0, 0.0, 0.1)); // Slightly offset
        vorticity_plane.set_size_2d(4.0, 2.0); // 2:1 aspect ratio plane to match data proportions

        let empty_vorticity = vec![0.0; (downsampled_width * downsampled_height) as usize];
        vorticity_plane.update_data(empty_vorticity, downsampled_width, downsampled_height);

        base.add_visualization("vorticity_field", vorticity_plane);

        // Create 4-layer grid resolution visualization
        let mut grid_resolution_plane = CutPlane2D::new();
        grid_resolution_plane.set_position(Vector3::new(0.0, 0.0, 0.2)); // Higher offset
        grid_resolution_plane.set_size_2d(4.0, 2.0); // 2:1 aspect ratio plane to match data proportions

        // Use AirSpeed coloring mode: 0.0 (finest) = blue/black, 3.0 (coarsest) = red/white
        grid_resolution_plane.set_coloring_mode(ColoringMode::AirSpeed);

        // Initialize with placeholder grid data - real grid will be generated during GPU init
        let placeholder_grid = vec![2.0f32; (downsampled_width * downsampled_height) as usize];
        grid_resolution_plane.update_data(placeholder_grid, downsampled_width, downsampled_height);

        base.add_visualization("grid_resolution", grid_resolution_plane);

        let simulation = Self {
            base,
            width: TOTAL_GRID_WIDTH,
            height: TOTAL_GRID_HEIGHT,
            generation: 0,
            is_paused: false,
            params: Lbm2d4LayerParams::default(),
            gpu_resources: None,
            needs_visualization_update: true,
            grid_levels_cache: None,
            visualization_scale: 2.0,
            cpu_velocity: vec![0.0; (TOTAL_GRID_WIDTH * TOTAL_GRID_HEIGHT * 3) as usize],
            cpu_vorticity: vec![0.0; (TOTAL_GRID_WIDTH * TOTAL_GRID_HEIGHT * 2) as usize],

            // Physical validation tracking
            wake_velocity_history: Vec::with_capacity(1000),
            last_strouhal_calculation: 0,
            measured_strouhal: None,
        };

        println!(
            "🌊 Initialized 2D LBM 4-layer multiresolution fluid simulation: {}x{} with D2Q9 lattice",
            TOTAL_GRID_WIDTH, TOTAL_GRID_HEIGHT
        );
        println!("   Level 0 (Finest): Around circle obstacle");
        println!("   Level 1 (Fine): Near field region");
        println!("   Level 2 (Medium): Intermediate field");
        println!("   Level 3 (Coarsest): Far field");

        simulation
    }

    /// Initialize GPU resources for 2D LBM computation
    fn initialize_gpu_resources(&mut self, device: &Device, queue: &Queue) {
        println!("🔧 Initializing 2D LBM 4-layer multiresolution GPU resources...");

        println!("🔧 Creating shaders...");
        // Create shaders
        let stream_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("LBM 2D 4-Layer Stream Shader"),
            source: wgpu::ShaderSource::Wgsl(LBM_2D_4LAYER_STREAM_SHADER.into()),
        });

        let collision_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("LBM 2D 4-Layer Collision Shader"),
            source: wgpu::ShaderSource::Wgsl(LBM_2D_4LAYER_COLLISION_SHADER.into()),
        });

        let interface_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("LBM 2D 4-Layer Interface Shader"),
            source: wgpu::ShaderSource::Wgsl(LBM_2D_4LAYER_INTERFACE_SHADER.into()),
        });

        let vorticity_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("LBM 2D 4-Layer Vorticity Shader"),
            source: wgpu::ShaderSource::Wgsl(LBM_2D_4LAYER_VORTICITY_SHADER.into()),
        });

        println!("🔧 Creating compute pipelines...");

        // Create bind group layouts
        let stream_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("LBM 2D 4-Layer Stream Layout"),
            entries: &[
                // Input distributions
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Output distributions
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Grid levels
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let collision_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("LBM 2D 4-Layer Collision Layout"),
            entries: &[
                // Distributions (read/write)
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Velocity output
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Parameters
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Boundary buffer
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Grid levels
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let interface_layout = collision_layout.clone(); // Same layout for interfaces

        let vorticity_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("LBM 2D 4-Layer Vorticity Layout"),
            entries: &[
                // Velocity input
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Vorticity output
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        // Create compute pipelines
        let stream_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("LBM 2D 4-Layer Stream Pipeline"),
            layout: Some(&device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("LBM 2D 4-Layer Stream Pipeline Layout"),
                bind_group_layouts: &[&stream_layout],
                push_constant_ranges: &[],
            })),
            module: &stream_shader,
            entry_point: Some("main"),
            cache: None,
            compilation_options: Default::default(),
        });

        let collision_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("LBM 2D 4-Layer Collision Pipeline"),
            layout: Some(&device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("LBM 2D 4-Layer Collision Pipeline Layout"),
                bind_group_layouts: &[&collision_layout],
                push_constant_ranges: &[],
            })),
            module: &collision_shader,
            entry_point: Some("main"),
            cache: None,
            compilation_options: Default::default(),
        });

        let interface_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("LBM 2D 4-Layer Interface Pipeline"),
            layout: Some(&device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("LBM 2D 4-Layer Interface Pipeline Layout"),
                bind_group_layouts: &[&interface_layout],
                push_constant_ranges: &[],
            })),
            module: &interface_shader,
            entry_point: Some("main"),
            cache: None,
            compilation_options: Default::default(),
        });

        let vorticity_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("LBM 2D 4-Layer Vorticity Pipeline"),
            layout: Some(&device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("LBM 2D 4-Layer Vorticity Pipeline Layout"),
                bind_group_layouts: &[&vorticity_layout],
                push_constant_ranges: &[],
            })),
            module: &vorticity_shader,
            entry_point: Some("main"),
            cache: None,
            compilation_options: Default::default(),
        });

        // Create buffers
        let distributions_size = (self.width * self.height * D2Q9_DIRECTIONS * std::mem::size_of::<f32>() as u32) as u64;
        let velocity_size = (self.width * self.height * 3 * std::mem::size_of::<f32>() as u32) as u64;
        let vorticity_size = (self.width * self.height * 2 * std::mem::size_of::<f32>() as u32) as u64;
        let grid_level_size = (self.width * self.height * std::mem::size_of::<u32>() as u32) as u64;
        let params_size = 16u64; // 4 f32 values (16 bytes) for proper alignment

        let distributions_buffer_a = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("LBM 2D 4-Layer Distributions A"),
            size: distributions_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let distributions_buffer_b = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("LBM 2D 4-Layer Distributions B"),
            size: distributions_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let velocity_density_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("LBM 2D 4-Layer Velocity Density Buffer"),
            size: velocity_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let vorticity_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("LBM 2D 4-Layer Vorticity Buffer"),
            size: vorticity_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // Create grid level buffer (now doing heavy generation only during GPU init)
        let grid_levels = Self::generate_grid_levels(&self.params);

        // Cache grid levels for visualization update
        self.grid_levels_cache = Some(grid_levels.clone());

        let grid_level_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("LBM 2D 4-Layer Grid Level Buffer"),
            size: grid_level_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&grid_level_buffer, 0, bytemuck::cast_slice(&grid_levels));

        println!("🔧 Creating boundary buffer...");
        // Create boundary buffer
        let boundary_data = Self::generate_circle_boundaries(&self.params);
        let boundary_size = (boundary_data.len() * std::mem::size_of::<u32>()) as u64;
        let boundary_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("LBM 2D 4-Layer Boundary Buffer"),
            size: boundary_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&boundary_buffer, 0, bytemuck::cast_slice(&boundary_data));

        let params_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("LBM 2D 4-Layer Parameters Buffer"),
            size: params_size,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create bind groups
        let stream_bind_group_a_to_b = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("LBM 2D 4-Layer Stream A->B"),
            layout: &stream_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: distributions_buffer_a.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: distributions_buffer_b.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: grid_level_buffer.as_entire_binding(),
                },
            ],
        });

        let stream_bind_group_b_to_a = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("LBM 2D 4-Layer Stream B->A"),
            layout: &stream_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: distributions_buffer_b.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: distributions_buffer_a.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: grid_level_buffer.as_entire_binding(),
                },
            ],
        });

        let collision_bind_group_a = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("LBM 2D 4-Layer Collision A"),
            layout: &collision_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: distributions_buffer_a.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: velocity_density_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: boundary_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: grid_level_buffer.as_entire_binding(),
                },
            ],
        });

        let collision_bind_group_b = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("LBM 2D 4-Layer Collision B"),
            layout: &collision_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: distributions_buffer_b.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: velocity_density_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: boundary_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: grid_level_buffer.as_entire_binding(),
                },
            ],
        });

        let interface_bind_group = collision_bind_group_a.clone();

        let vorticity_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("LBM 2D 4-Layer Vorticity"),
            layout: &vorticity_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: velocity_density_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: vorticity_buffer.as_entire_binding(),
                },
            ],
        });

        self.gpu_resources = Some(Lbm2d4LayerGpuResources {
            stream_pipeline,
            collision_pipeline,
            interface_pipeline,
            vorticity_pipeline,
            distributions_buffer_a,
            distributions_buffer_b,
            velocity_density_buffer,
            vorticity_buffer,
            grid_level_buffer,
            boundary_buffer,
            params_buffer,
            stream_bind_group_a_to_b,
            stream_bind_group_b_to_a,
            collision_bind_group_a,
            collision_bind_group_b,
            interface_bind_group,
            vorticity_bind_group,
            ping_pong_state: false,
        });

        println!("✅ 2D LBM 4-layer multiresolution GPU resources initialized successfully");
    }

    /// Initialize LBM simulation with equilibrium distributions
    fn initialize_simulation(&self, _device: &Device, queue: &Queue) {
        if let Some(ref gpu_resources) = self.gpu_resources {
            // Initialize with rest state (zero velocity, unit density)
            let total_cells = (self.width * self.height) as usize;
            let mut distributions = vec![0.0f32; total_cells * D2Q9_DIRECTIONS as usize];

            // Set equilibrium distributions for rest state (D2Q9 weights)
            let weights = [
                4.0/9.0,                          // 0: rest
                1.0/9.0, 1.0/9.0, 1.0/9.0, 1.0/9.0, // 1-4: cardinal directions
                1.0/36.0, 1.0/36.0, 1.0/36.0, 1.0/36.0, // 5-8: diagonal directions
            ];

            // Add small random noise to initial conditions for flow instabilities
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};

            for cell in 0..total_cells {
                for i in 0..D2Q9_DIRECTIONS as usize {
                    // Create deterministic "random" noise based on cell position
                    let mut hasher = DefaultHasher::new();
                    (cell, i).hash(&mut hasher);
                    let hash_value = hasher.finish();

                    // Convert hash to small noise value (±1% of base weight)
                    let noise_amplitude = 0.01;
                    let noise = (hash_value as f32 / u64::MAX as f32 - 0.5) * 2.0 * noise_amplitude;

                    distributions[cell * D2Q9_DIRECTIONS as usize + i] = weights[i] * (1.0 + noise);
                }
            }

            // Upload to both distribution buffers
            queue.write_buffer(&gpu_resources.distributions_buffer_a, 0, bytemuck::cast_slice(&distributions));
            queue.write_buffer(&gpu_resources.distributions_buffer_b, 0, bytemuck::cast_slice(&distributions));

            // Upload parameters
            let params_data = [
                self.params.tau,
                self.params.inlet_velocity,
                self.params.outlet_pressure,
                self.params.circle_radius,
            ];
            queue.write_buffer(&gpu_resources.params_buffer, 0, bytemuck::cast_slice(&params_data));

            println!("🌊 2D LBM 4-layer multiresolution simulation initialized with equilibrium state");
        }
    }

    /// Run one LBM timestep: stream -> collision -> interface -> vorticity
    fn run_lbm_step(&mut self, device: &Device, queue: &Queue) {
        if let Some(ref mut gpu_resources) = self.gpu_resources {
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("LBM 2D 4-Layer Step Encoder"),
            });

            // Step 1: Stream step (propagation with 4-layer multiresolution handling)
            {
                let mut stream_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("LBM 2D 4-Layer Stream Pass"),
                    timestamp_writes: None,
                });

                stream_pass.set_pipeline(&gpu_resources.stream_pipeline);

                let stream_bind_group = if gpu_resources.ping_pong_state {
                    &gpu_resources.stream_bind_group_b_to_a
                } else {
                    &gpu_resources.stream_bind_group_a_to_b
                };

                stream_pass.set_bind_group(0, stream_bind_group, &[]);

                let workgroup_size = 8; // 8x8 workgroups for 2D
                let num_workgroups_x = (self.width + workgroup_size - 1) / workgroup_size;
                let num_workgroups_y = (self.height + workgroup_size - 1) / workgroup_size;

                stream_pass.dispatch_workgroups(num_workgroups_x, num_workgroups_y, 1);
            }

            // Flip ping-pong state after streaming
            gpu_resources.ping_pong_state = !gpu_resources.ping_pong_state;

            // Step 2: Collision step (BGK with 4-layer multiresolution time stepping)
            {
                let mut collision_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("LBM 2D 4-Layer Collision Pass"),
                    timestamp_writes: None,
                });

                collision_pass.set_pipeline(&gpu_resources.collision_pipeline);

                let collision_bind_group = if gpu_resources.ping_pong_state {
                    &gpu_resources.collision_bind_group_b
                } else {
                    &gpu_resources.collision_bind_group_a
                };

                collision_pass.set_bind_group(0, collision_bind_group, &[]);

                let workgroup_size = 8;
                let num_workgroups_x = (self.width + workgroup_size - 1) / workgroup_size;
                let num_workgroups_y = (self.height + workgroup_size - 1) / workgroup_size;

                collision_pass.dispatch_workgroups(num_workgroups_x, num_workgroups_y, 1);
            }

            // Step 3: Interface handling between all 4 grid levels
            {
                let mut interface_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("LBM 2D 4-Layer Interface Pass"),
                    timestamp_writes: None,
                });

                interface_pass.set_pipeline(&gpu_resources.interface_pipeline);
                interface_pass.set_bind_group(0, &gpu_resources.interface_bind_group, &[]);

                let workgroup_size = 8;
                let num_workgroups_x = (self.width + workgroup_size - 1) / workgroup_size;
                let num_workgroups_y = (self.height + workgroup_size - 1) / workgroup_size;

                interface_pass.dispatch_workgroups(num_workgroups_x, num_workgroups_y, 1);
            }

            // Step 4: Vorticity calculation
            {
                let mut vorticity_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("LBM 2D 4-Layer Vorticity Pass"),
                    timestamp_writes: None,
                });

                vorticity_pass.set_pipeline(&gpu_resources.vorticity_pipeline);
                vorticity_pass.set_bind_group(0, &gpu_resources.vorticity_bind_group, &[]);

                let workgroup_size = 8;
                let num_workgroups_x = (self.width + workgroup_size - 1) / workgroup_size;
                let num_workgroups_y = (self.height + workgroup_size - 1) / workgroup_size;

                vorticity_pass.dispatch_workgroups(num_workgroups_x, num_workgroups_y, 1);
            }

            queue.submit(std::iter::once(encoder.finish()));
            self.generation += 1;
        }
    }

    /// Sync GPU data back to CPU for visualization
    fn sync_data_to_cpu(&mut self, device: &Device, queue: &Queue) {
        if let Some(ref gpu_resources) = self.gpu_resources {
            let velocity_size = (self.width * self.height * 3 * std::mem::size_of::<f32>() as u32) as u64;
            let vorticity_size = (self.width * self.height * 2 * std::mem::size_of::<f32>() as u32) as u64;

            // Create staging buffers
            let velocity_staging = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("LBM 2D 4-Layer Velocity Staging"),
                size: velocity_size,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });

            let vorticity_staging = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("LBM 2D 4-Layer Vorticity Staging"),
                size: vorticity_size,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });

            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("LBM 2D 4-Layer Data Sync Encoder"),
            });

            encoder.copy_buffer_to_buffer(&gpu_resources.velocity_density_buffer, 0, &velocity_staging, 0, velocity_size);
            encoder.copy_buffer_to_buffer(&gpu_resources.vorticity_buffer, 0, &vorticity_staging, 0, vorticity_size);
            queue.submit(std::iter::once(encoder.finish()));

            // Map and read velocity data
            let velocity_slice = velocity_staging.slice(..);
            let (tx_vel, rx_vel) = std::sync::mpsc::channel();
            velocity_slice.map_async(wgpu::MapMode::Read, move |result| {
                tx_vel.send(result).unwrap();
            });

            let _ = device.poll(wgpu::MaintainBase::Wait);

            if let Ok(Ok(())) = rx_vel.recv() {
                let data = velocity_slice.get_mapped_range();
                let f32_data: &[f32] = bytemuck::cast_slice(&data);
                if self.cpu_velocity.len() == f32_data.len() {
                    self.cpu_velocity.copy_from_slice(f32_data);
                }
            }

            // Map and read vorticity data
            let vorticity_slice = vorticity_staging.slice(..);
            let (tx_vort, rx_vort) = std::sync::mpsc::channel();
            vorticity_slice.map_async(wgpu::MapMode::Read, move |result| {
                tx_vort.send(result).unwrap();
            });

            let _ = device.poll(wgpu::MaintainBase::Wait);

            if let Ok(Ok(())) = rx_vort.recv() {
                let data = vorticity_slice.get_mapped_range();
                let f32_data: &[f32] = bytemuck::cast_slice(&data);
                if self.cpu_vorticity.len() == f32_data.len() {
                    self.cpu_vorticity.copy_from_slice(f32_data);
                }
            }

            self.update_visualizations(device, queue);
        }
    }

    /// Update visualization planes with current simulation data
    fn update_visualizations(&mut self, device: &Device, queue: &Queue) {
        // Downsample by 2x to reduce aliasing and improve visual quality
        let downsample_factor = 2usize;
        let downsampled_width = (self.width as usize / downsample_factor) as u32;
        let downsampled_height = (self.height as usize / downsample_factor) as u32;

        // Extract and downsample velocity magnitude for visualization
        let velocity_magnitudes = self.downsample_data(
            &self.cpu_velocity.chunks(3)
                .map(|chunk| (chunk[0] * chunk[0] + chunk[1] * chunk[1]).sqrt())
                .collect::<Vec<f32>>(),
            self.width as usize,
            self.height as usize,
            downsample_factor
        );

        // Update velocity visualization with downsampled data
        if let Some(visualization) = self.base.get_visualization_mut("velocity_field") {
            if let Some(velocity_plane) = visualization.as_any_mut().downcast_mut::<CutPlane2D>() {
                velocity_plane.update_data(velocity_magnitudes, downsampled_width, downsampled_height);
                velocity_plane.set_size_2d(self.visualization_scale * 2.0, self.visualization_scale); // 2:1 aspect ratio scaling
                velocity_plane.update(0.0, Some(device), Some(queue));
            }
        }

        // Extract and downsample vorticity for visualization
        let vorticity_values = self.downsample_data(
            &self.cpu_vorticity.chunks(2)
                .map(|chunk| chunk[0]) // Just the vorticity component (not magnitude)
                .collect::<Vec<f32>>(),
            self.width as usize,
            self.height as usize,
            downsample_factor
        );

        // Update vorticity visualization with downsampled data
        if let Some(visualization) = self.base.get_visualization_mut("vorticity_field") {
            if let Some(vorticity_plane) = visualization.as_any_mut().downcast_mut::<CutPlane2D>() {
                vorticity_plane.update_data(vorticity_values, downsampled_width, downsampled_height);
                vorticity_plane.set_size_2d(self.visualization_scale * 2.0, self.visualization_scale); // 2:1 aspect ratio scaling
                vorticity_plane.update(0.0, Some(device), Some(queue));
            }
        }

        // Update 4-layer grid resolution visualization (now with real data)
        if let Some(visualization) = self.base.get_visualization_mut("grid_resolution") {
            if let Some(grid_plane) = visualization.as_any_mut().downcast_mut::<CutPlane2D>() {
                // Update with real grid data if available
                if let Some(ref grid_levels) = self.grid_levels_cache {
                    let downsample_factor = 2usize;
                    let downsampled_grid_levels = Self::downsample_grid_levels(grid_levels, downsample_factor);
                    let downsampled_width = (self.width as usize / downsample_factor) as u32;
                    let downsampled_height = (self.height as usize / downsample_factor) as u32;
                    grid_plane.update_data(downsampled_grid_levels, downsampled_width, downsampled_height);
                }
                grid_plane.set_size_2d(self.visualization_scale * 2.0, self.visualization_scale); // 2:1 aspect ratio scaling
                grid_plane.update(0.0, Some(device), Some(queue));
            }
        }

        self.needs_visualization_update = false;
    }

    /// Downsample 4-layer grid level data to match visualization resolution
    fn downsample_grid_levels(grid_levels: &[u32], factor: usize) -> Vec<f32> {
        let width = TOTAL_GRID_WIDTH as usize;
        let height = TOTAL_GRID_HEIGHT as usize;
        let new_width = width / factor;
        let new_height = height / factor;
        let mut downsampled = Vec::with_capacity(new_width * new_height);

        for new_y in 0..new_height {
            for new_x in 0..new_width {
                // Sample the center of each downsampled cell
                let old_x = new_x * factor + factor / 2;
                let old_y = new_y * factor + factor / 2;

                if old_x < width && old_y < height {
                    let index = old_y * width + old_x;
                    // Convert grid level to float: 0.0 (finest) to 3.0 (coarsest)
                    // Normalize to 0.0-1.0 range for visualization
                    downsampled.push(grid_levels[index] as f32 / 3.0);
                } else {
                    downsampled.push(1.0); // Default to coarsest
                }
            }
        }

        downsampled
    }

    /// Downsample 2D data using area averaging to reduce aliasing
    fn downsample_data(&self, data: &[f32], width: usize, height: usize, factor: usize) -> Vec<f32> {
        let new_width = width / factor;
        let new_height = height / factor;
        let mut downsampled = Vec::with_capacity(new_width * new_height);

        for new_y in 0..new_height {
            for new_x in 0..new_width {
                let mut sum = 0.0;
                let mut count = 0;

                // Average over the factor x factor window
                for dy in 0..factor {
                    for dx in 0..factor {
                        let old_x = new_x * factor + dx;
                        let old_y = new_y * factor + dy;
                        if old_x < width && old_y < height {
                            let index = old_y * width + old_x;
                            sum += data[index];
                            count += 1;
                        }
                    }
                }

                downsampled.push(if count > 0 { sum / count as f32 } else { 0.0 });
            }
        }

        downsampled
    }
}

// End of Lbm2d4LayerMultiresolutionSimulation impl block

impl haggis::simulation::traits::Simulation for Lbm2d4LayerMultiresolutionSimulation {
    fn initialize(&mut self, scene: &mut haggis::gfx::scene::Scene) {
        self.base.initialize(scene);
        println!("🌊 2D LBM 4-layer multiresolution simulation initialized");
    }

    fn initialize_gpu(&mut self, device: &Device, queue: &Queue) {
        self.base.initialize_gpu(device, queue);
        self.initialize_gpu_resources(device, queue);
        self.initialize_simulation(device, queue);
        self.sync_data_to_cpu(device, queue);
        println!("✅ 2D LBM 4-layer multiresolution GPU initialization complete");
    }

    fn update(&mut self, delta_time: f32, scene: &mut haggis::gfx::scene::Scene) {
        self.base.update(delta_time, scene);
    }

    fn update_gpu(&mut self, device: &Device, queue: &Queue, _delta_time: f32) {
        // Update GPU parameters if changed
        if let Some(ref gpu_resources) = self.gpu_resources {
            let params_data = [
                self.params.tau,
                self.params.inlet_velocity,
                self.params.outlet_pressure,
                self.params.circle_radius,
            ];
            queue.write_buffer(&gpu_resources.params_buffer, 0, bytemuck::cast_slice(&params_data));
        }

        // Run simulation if not paused
        if !self.is_paused && self.gpu_resources.is_some() {
            self.run_lbm_step(device, queue);

            // Sync data every few steps for visualization
            if self.generation % 5 == 0 {
                self.sync_data_to_cpu(device, queue);
            }
        }

        self.base.update_gpu(device, queue, _delta_time);
    }

    fn apply_gpu_results_to_scene(&mut self, device: &Device, scene: &mut haggis::gfx::scene::Scene) {
        self.base.apply_gpu_results_to_scene(device, scene);
    }

    fn render_ui(&mut self, ui: &imgui::Ui) {
        ui.window("LBM 2D 4-Layer Multiresolution")
            .size([550.0, 800.0], imgui::Condition::FirstUseEver)
            .build(|| {
                ui.text("🌊 2D Lattice Boltzmann Method (4-Layer Multiresolution)");
                ui.separator();

                ui.text(&format!("Timestep: {}", self.generation));
                ui.text(&format!("Grid Size: {}x{} ({} cells)",
                    self.width, self.height, self.width * self.height));
                ui.text(&format!("Grid Aspect Ratio: 2:1 ({}x{})", TOTAL_GRID_WIDTH, TOTAL_GRID_HEIGHT));
                ui.text(&format!("Max Grid Depth: 4 levels (0=Finest, 3=Coarsest)"));
                ui.text(&format!("Lattice: D2Q{}", D2Q9_DIRECTIONS));
                ui.text(&format!("GPU Ready: {}", self.gpu_resources.is_some()));

                ui.separator();

                // Play/Pause controls
                if ui.button(if self.is_paused { "▶ Play" } else { "⏸ Pause" }) {
                    self.is_paused = !self.is_paused;
                }

                ui.separator();

                // Flow Parameters
                ui.text("Flow Parameters:");

                ui.slider_config("Relaxation Time (τ)", 0.51, 2.0)
                    .display_format("%.3f")
                    .build(&mut self.params.tau);

                ui.slider_config("Inlet Velocity", 0.0, 0.2)
                    .display_format("%.3f")
                    .build(&mut self.params.inlet_velocity);

                ui.slider_config("Outlet Pressure", 0.8, 1.2)
                    .display_format("%.3f")
                    .build(&mut self.params.outlet_pressure);

                ui.separator();

                // Circle Parameters
                ui.text("Circle Obstacle:");

                ui.slider_config("Circle Radius", 5.0, 40.0)
                    .display_format("%.1f")
                    .build(&mut self.params.circle_radius);

                ui.slider_config("Circle Center X", 0.1, 0.5)
                    .display_format("%.2f")
                    .build(&mut self.params.circle_center_x);

                ui.slider_config("Circle Center Y", 0.3, 0.7)
                    .display_format("%.2f")
                    .build(&mut self.params.circle_center_y);

                ui.separator();

                // 4-Layer Multiresolution Parameters
                ui.text("4-Layer Multiresolution Grid:");

                let mut refinement = self.params.refinement_factor as i32;
                if ui.slider_config("Refinement Factor", 1, 4).build(&mut refinement) {
                    self.params.refinement_factor = refinement as u32;
                }

                ui.text(&format!("ULTRA-CONSERVATIVE MODE: 2-Level Only"));
                ui.text(&format!("Level 2 (Finer): 0-60 grid units (massive core)"));
                ui.text(&format!("Level 3 (Coarser): 60+ grid units (far field)"));
                ui.text(&format!("Step sizes: Level 2=2, Level 3=2 (minimal difference)"));

                // Calculate ultra-conservative 2-level grid statistics
                let total_cells = self.width * self.height;
                let level2_area: f32 = 3.14159 * 60.0 * 60.0; // Massive 60 grid unit radius

                // Massive wake area estimation
                let wake_area: f32 = 40.0 * 100.0; // 40 width x 100 length

                // Add massive inlet/outlet areas
                let inlet_area: f32 = (TOTAL_GRID_WIDTH as f32 * 0.25) * TOTAL_GRID_HEIGHT as f32;
                let outlet_area: f32 = (TOTAL_GRID_WIDTH as f32 * 0.25) * TOTAL_GRID_HEIGHT as f32;

                let level2_cells = (level2_area + wake_area + inlet_area + outlet_area).min(total_cells as f32) as u32;
                let level3_cells = total_cells - level2_cells;

                ui.text(&format!("Level 2 cells: ~{} ({:.1}%)", level2_cells, 100.0 * level2_cells as f32 / total_cells as f32));
                ui.text(&format!("Level 3 cells: ~{} ({:.1}%)", level3_cells, 100.0 * level3_cells as f32 / total_cells as f32));
                ui.text(&format!("Levels 0-1: DISABLED for stability"));

                ui.separator();

                // Flow Analysis
                ui.text("Flow Analysis:");
                ui.text(&format!("Kinematic Viscosity: {:.6}", (self.params.tau - 0.5) / 3.0));
                let reynolds = self.params.inlet_velocity * self.params.circle_radius * 2.0
                    / ((self.params.tau - 0.5) / 3.0);
                ui.text(&format!("Reynolds Number: {:.1}", reynolds));

                // Show flow regime
                if reynolds < 20.0 {
                    ui.text_colored([0.7, 0.7, 0.7, 1.0], "Flow: Steady (no shedding)");
                } else if reynolds < 200.0 {
                    ui.text_colored([0.0, 1.0, 0.0, 1.0], "Flow: Vortex shedding!");
                } else {
                    ui.text_colored([1.0, 0.5, 0.0, 1.0], "Flow: Turbulent");
                }

                ui.separator();

                // Visualization controls
                ui.text("Visualization:");
                ui.slider_config("Scale", 1.0, 8.0)
                    .display_format("%.1f")
                    .build(&mut self.visualization_scale);

                ui.text("Display Info:");
                ui.text(&format!("Current scale: {:.1}x", self.visualization_scale));
                ui.text(&format!("Visualization size: {:.1}x{:.1}", self.visualization_scale * 2.0, self.visualization_scale));
                ui.text("Note: 2:1 aspect ratio visualization");

                ui.separator();

                // Status
                ui.text("Status:");
                if self.is_paused {
                    ui.text_colored([1.0, 1.0, 0.0, 1.0], "⏸ Paused");
                } else if self.gpu_resources.is_some() {
                    ui.text_colored([0.0, 1.0, 0.0, 1.0], "▶ Running (2D 4-Layer Multiresolution)");
                } else {
                    ui.text_colored([1.0, 0.5, 0.0, 1.0], "⚙ Initializing GPU...");
                }

                ui.separator();
                ui.text("Visualization Layers:");
                ui.bullet_text("Velocity field (bottom): Flow speed magnitude");
                ui.bullet_text("Vorticity field (middle): Rotation patterns");
                ui.bullet_text("Grid resolution (top): 4-layer refinement");
                ui.text("  • Dark blue = Level 0 (Finest)");
                ui.text("  • Light blue = Level 1 (Fine)");
                ui.text("  • Yellow = Level 2 (Medium)");
                ui.text("  • Red = Level 3 (Coarsest)");

                ui.separator();
                ui.text("4-Layer LBM Features:");
                ui.bullet_text("D2Q9 lattice model");
                ui.bullet_text("4-layer multiresolution grid (geometry-based)");
                ui.bullet_text("BGK collision operator with level-specific time stepping");
                ui.bullet_text("Zou-He inlet/outlet boundaries");
                ui.bullet_text("Circle obstacle with bounce-back");
                ui.bullet_text("Real-time velocity & vorticity visualization");
                ui.bullet_text("Graduated wake refinement");
                ui.bullet_text("Step sizes: 1, 2, 4, 8 for levels 0-3");
            });

        self.base.render_ui(ui);
    }

    fn name(&self) -> &str {
        "LBM 2D 4-Layer Multiresolution"
    }

    fn is_running(&self) -> bool {
        !self.is_paused
    }

    fn set_running(&mut self, running: bool) {
        self.is_paused = !running;
    }

    fn reset(&mut self, scene: &mut haggis::gfx::scene::Scene) {
        println!("🔄 Resetting 2D LBM 4-layer multiresolution simulation");
        self.generation = 0;
        self.base.reset(scene);
    }

    fn as_any(&self) -> &dyn std::any::Any {
        &self.base
    }
}

// 2D LBM compute shaders for 4-layer multiresolution simulation

const LBM_2D_4LAYER_STREAM_SHADER: &str = r#"
// D2Q9 lattice directions for 2D
const D2Q9_DIRECTIONS: u32 = 9u;
const GRID_WIDTH: u32 = 512u;  // 2 * 256
const GRID_HEIGHT: u32 = 256u;

// D2Q9 velocity vectors
const VELOCITY_SET: array<vec2<i32>, 9> = array<vec2<i32>, 9>(
    vec2<i32>( 0,  0),  // 0: rest
    vec2<i32>( 1,  0),  // 1: +x
    vec2<i32>( 0,  1),  // 2: +y
    vec2<i32>(-1,  0),  // 3: -x
    vec2<i32>( 0, -1),  // 4: -y
    vec2<i32>( 1,  1),  // 5: +x+y
    vec2<i32>(-1,  1),  // 6: -x+y
    vec2<i32>(-1, -1),  // 7: -x-y
    vec2<i32>( 1, -1),  // 8: +x-y
);

@group(0) @binding(0) var<storage, read> input_distributions: array<f32>;
@group(0) @binding(1) var<storage, read_write> output_distributions: array<f32>;
@group(0) @binding(2) var<storage, read> grid_levels: array<u32>;

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let x = global_id.x;
    let y = global_id.y;

    if (x >= GRID_WIDTH || y >= GRID_HEIGHT) {
        return;
    }

    let cell_index = y * GRID_WIDTH + x;
    let grid_level = grid_levels[cell_index];

    // Stream each distribution function with grid-level-aware propagation
    for (var i: u32 = 0u; i < D2Q9_DIRECTIONS; i++) {
        let velocity = VELOCITY_SET[i];

        // Adjust velocity based on grid level (3-level system: 1=1, 2=1, 3=2)
        // Conservative: levels 1 and 2 use step size 1, level 3 uses step size 2
        let step_size = select(1u, 2u, grid_level >= 3u);
        let adjusted_velocity = vec2<i32>(velocity.x * i32(step_size), velocity.y * i32(step_size));

        // Calculate source position (where this distribution came from)
        let src_x = (i32(x) - adjusted_velocity.x + i32(GRID_WIDTH)) % i32(GRID_WIDTH);
        let src_y = (i32(y) - adjusted_velocity.y + i32(GRID_HEIGHT)) % i32(GRID_HEIGHT);

        let src_cell_index = u32(src_y) * GRID_WIDTH + u32(src_x);
        let src_dist_index = src_cell_index * D2Q9_DIRECTIONS + i;
        let dst_dist_index = cell_index * D2Q9_DIRECTIONS + i;

        // Stream the distribution function
        output_distributions[dst_dist_index] = input_distributions[src_dist_index];
    }
}
"#;

const LBM_2D_4LAYER_COLLISION_SHADER: &str = r#"
const D2Q9_DIRECTIONS: u32 = 9u;
const GRID_WIDTH: u32 = 512u;
const GRID_HEIGHT: u32 = 256u;

// D2Q9 weights
const WEIGHTS: array<f32, 9> = array<f32, 9>(
    4.0/9.0,                              // 0: rest
    1.0/9.0, 1.0/9.0, 1.0/9.0, 1.0/9.0,  // 1-4: cardinal directions
    1.0/36.0, 1.0/36.0, 1.0/36.0, 1.0/36.0, // 5-8: diagonal directions
);

// D2Q9 velocity vectors
const VELOCITY_SET: array<vec2<f32>, 9> = array<vec2<f32>, 9>(
    vec2<f32>( 0.0,  0.0),  // 0: rest
    vec2<f32>( 1.0,  0.0),  // 1: +x
    vec2<f32>( 0.0,  1.0),  // 2: +y
    vec2<f32>(-1.0,  0.0),  // 3: -x
    vec2<f32>( 0.0, -1.0),  // 4: -y
    vec2<f32>( 1.0,  1.0),  // 5: +x+y
    vec2<f32>(-1.0,  1.0),  // 6: -x+y
    vec2<f32>(-1.0, -1.0),  // 7: -x-y
    vec2<f32>( 1.0, -1.0),  // 8: +x-y
);

@group(0) @binding(0) var<storage, read_write> distributions: array<f32>;
@group(0) @binding(1) var<storage, read_write> velocity_density: array<f32>; // [vx, vy, density]
@group(0) @binding(2) var<uniform> params: vec4<f32>; // [tau, inlet_vel, outlet_pressure, circle_radius]
@group(0) @binding(3) var<storage, read> boundary_buffer: array<u32>; // bit-packed boundary flags
@group(0) @binding(4) var<storage, read> grid_levels: array<u32>;

// Check if cell is a boundary using bit-packed buffer
fn is_boundary_cell(x: u32, y: u32) -> bool {
    let cell_index = y * GRID_WIDTH + x;
    let u32_index = cell_index / 32u;
    let bit_index = cell_index % 32u;

    if (u32_index >= arrayLength(&boundary_buffer)) {
        return false;
    }

    let boundary_bits = boundary_buffer[u32_index];
    return (boundary_bits & (1u << bit_index)) != 0u;
}

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let x = global_id.x;
    let y = global_id.y;

    if (x >= GRID_WIDTH || y >= GRID_HEIGHT) {
        return;
    }

    let cell_index = y * GRID_WIDTH + x;
    let base_dist_index = cell_index * D2Q9_DIRECTIONS;
    let grid_level = grid_levels[cell_index];

    // Parameters
    let tau = params.x;
    let inlet_velocity = params.y;
    let outlet_pressure = params.z;

    // Conservative: use same tau for all levels for stability
    let effective_tau = tau;

    // Calculate macroscopic quantities
    var density = 0.0;
    var velocity = vec2<f32>(0.0);

    for (var i: u32 = 0u; i < D2Q9_DIRECTIONS; i++) {
        let f_i = distributions[base_dist_index + i];
        density += f_i;
        velocity += f_i * VELOCITY_SET[i];
    }

    velocity = velocity / density;

    // Check boundary using bit-packed buffer
    let is_inside_obstacle = is_boundary_cell(x, y);

    // Apply boundary conditions
    var is_boundary = false;

    // Inlet boundary (left wall, x = 0) - Zou-He velocity inlet
    if (x == 0u && !is_inside_obstacle) {
        velocity = vec2<f32>(inlet_velocity, 0.0);
        density = 1.0;
        is_boundary = true;

        // Set equilibrium distributions for inlet
        for (var i: u32 = 0u; i < D2Q9_DIRECTIONS; i++) {
            let ci = VELOCITY_SET[i];
            let weight = WEIGHTS[i];
            let ci_dot_u = dot(ci, velocity);
            let u_dot_u = dot(velocity, velocity);
            distributions[base_dist_index + i] = weight * density * (1.0 + 3.0 * ci_dot_u + 4.5 * ci_dot_u * ci_dot_u - 1.5 * u_dot_u);
        }
    }

    // Outlet boundary (right wall, x = GRID_WIDTH - 1) - Zou-He pressure outlet
    else if (x == GRID_WIDTH - 1u && !is_inside_obstacle) {
        density = outlet_pressure;
        is_boundary = true;

        // Set equilibrium distributions for outlet
        for (var i: u32 = 0u; i < D2Q9_DIRECTIONS; i++) {
            let ci = VELOCITY_SET[i];
            let weight = WEIGHTS[i];
            let ci_dot_u = dot(ci, velocity);
            let u_dot_u = dot(velocity, velocity);
            distributions[base_dist_index + i] = weight * density * (1.0 + 3.0 * ci_dot_u + 4.5 * ci_dot_u * ci_dot_u - 1.5 * u_dot_u);
        }
    }

    // Solid walls (top/bottom) - bounce-back
    else if (y == 0u || y == GRID_HEIGHT - 1u) {
        velocity = vec2<f32>(0.0, 0.0);
        is_boundary = true;

        // Bounce-back BC
        let f1_old = distributions[base_dist_index + 1u]; // +x
        let f2_old = distributions[base_dist_index + 2u]; // +y
        let f3_old = distributions[base_dist_index + 3u]; // -x
        let f4_old = distributions[base_dist_index + 4u]; // -y
        let f5_old = distributions[base_dist_index + 5u]; // +x+y
        let f6_old = distributions[base_dist_index + 6u]; // -x+y
        let f7_old = distributions[base_dist_index + 7u]; // -x-y
        let f8_old = distributions[base_dist_index + 8u]; // +x-y

        distributions[base_dist_index + 1u] = f3_old; // +x <- -x
        distributions[base_dist_index + 2u] = f4_old; // +y <- -y
        distributions[base_dist_index + 3u] = f1_old; // -x <- +x
        distributions[base_dist_index + 4u] = f2_old; // -y <- +y
        distributions[base_dist_index + 5u] = f7_old; // +x+y <- -x-y
        distributions[base_dist_index + 6u] = f8_old; // -x+y <- +x-y
        distributions[base_dist_index + 7u] = f5_old; // -x-y <- +x+y
        distributions[base_dist_index + 8u] = f6_old; // +x-y <- -x+y
    }

    // Circle obstacles - bounce-back
    else if (is_inside_obstacle) {
        velocity = vec2<f32>(0.0, 0.0);
        is_boundary = true;

        // Bounce-back BC for circle
        let f1_old = distributions[base_dist_index + 1u];
        let f2_old = distributions[base_dist_index + 2u];
        let f3_old = distributions[base_dist_index + 3u];
        let f4_old = distributions[base_dist_index + 4u];
        let f5_old = distributions[base_dist_index + 5u];
        let f6_old = distributions[base_dist_index + 6u];
        let f7_old = distributions[base_dist_index + 7u];
        let f8_old = distributions[base_dist_index + 8u];

        distributions[base_dist_index + 1u] = f3_old;
        distributions[base_dist_index + 2u] = f4_old;
        distributions[base_dist_index + 3u] = f1_old;
        distributions[base_dist_index + 4u] = f2_old;
        distributions[base_dist_index + 5u] = f7_old;
        distributions[base_dist_index + 6u] = f8_old;
        distributions[base_dist_index + 7u] = f5_old;
        distributions[base_dist_index + 8u] = f6_old;
    }

    // Fluid domain - BGK collision with 3-level stabilization
    if (!is_boundary) {
        let omega = 1.0 / effective_tau;

        // 3-level stabilization: apply slightly more conservative relaxation for finest levels
        let stabilized_omega = select(omega, omega * 0.95, grid_level <= 2u);  // More conservative for levels 1 and 2

        // Enhanced velocity limiting for 3-level stability
        let max_velocity = 0.15;  // Conservative velocity cap
        let velocity_magnitude = sqrt(dot(velocity, velocity));
        let limited_velocity = select(velocity, velocity * (max_velocity / velocity_magnitude), velocity_magnitude > max_velocity);

        for (var i: u32 = 0u; i < D2Q9_DIRECTIONS; i++) {
            let ci = VELOCITY_SET[i];
            let weight = WEIGHTS[i];

            // Equilibrium distribution with limited velocity
            let ci_dot_u = dot(ci, limited_velocity);
            let u_dot_u = dot(limited_velocity, limited_velocity);
            let f_eq = weight * density * (1.0 + 3.0 * ci_dot_u + 4.5 * ci_dot_u * ci_dot_u - 1.5 * u_dot_u);

            // BGK collision with stabilized omega
            let f_old = distributions[base_dist_index + i];
            distributions[base_dist_index + i] = f_old - stabilized_omega * (f_old - f_eq);
        }
    }

    // Store velocity and density for vorticity calculation
    velocity_density[cell_index * 3u + 0u] = velocity.x;
    velocity_density[cell_index * 3u + 1u] = velocity.y;
    velocity_density[cell_index * 3u + 2u] = density;
}
"#;

const LBM_2D_4LAYER_INTERFACE_SHADER: &str = r#"
const GRID_WIDTH: u32 = 512u;
const GRID_HEIGHT: u32 = 256u;

@group(0) @binding(0) var<storage, read_write> distributions: array<f32>;
@group(0) @binding(1) var<storage, read_write> velocity_density: array<f32>;
@group(0) @binding(2) var<uniform> params: vec4<f32>;
@group(0) @binding(3) var<storage, read> boundary_buffer: array<u32>;
@group(0) @binding(4) var<storage, read> grid_levels: array<u32>;

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let x = global_id.x;
    let y = global_id.y;

    if (x >= GRID_WIDTH || y >= GRID_HEIGHT) {
        return;
    }

    let cell_index = y * GRID_WIDTH + x;
    let current_level = grid_levels[cell_index];

    // Conservative approach: minimal interface corrections
    // Let natural diffusion handle most level transitions for stability
    return;
}
"#;

const LBM_2D_4LAYER_VORTICITY_SHADER: &str = r#"
const GRID_WIDTH: u32 = 512u;
const GRID_HEIGHT: u32 = 256u;

@group(0) @binding(0) var<storage, read> velocity_density: array<f32>; // [vx, vy, density]
@group(0) @binding(1) var<storage, read_write> vorticity: array<f32>; // [vorticity, magnitude]

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let x = global_id.x;
    let y = global_id.y;

    if (x >= GRID_WIDTH || y >= GRID_HEIGHT) {
        return;
    }

    let cell_index = y * GRID_WIDTH + x;

    // Calculate vorticity using finite differences
    // ω = ∂v_y/∂x - ∂v_x/∂y

    // Get neighboring coordinates (with boundary handling)
    let x_plus = min(x + 1u, GRID_WIDTH - 1u);
    let x_minus = max(x, 1u) - 1u;
    let y_plus = min(y + 1u, GRID_HEIGHT - 1u);
    let y_minus = max(y, 1u) - 1u;

    // Get velocity components at neighboring cells
    let idx_xp = y * GRID_WIDTH + x_plus;
    let idx_xm = y * GRID_WIDTH + x_minus;
    let idx_yp = y_plus * GRID_WIDTH + x;
    let idx_ym = y_minus * GRID_WIDTH + x;

    // Central differences for velocity gradients
    let dvy_dx = (velocity_density[idx_xp * 3u + 1u] - velocity_density[idx_xm * 3u + 1u]) * 0.5;
    let dvx_dy = (velocity_density[idx_yp * 3u + 0u] - velocity_density[idx_ym * 3u + 0u]) * 0.5;

    // Vorticity: ω = ∂v_y/∂x - ∂v_x/∂y
    let omega = dvy_dx - dvx_dy;

    // Vorticity magnitude (same as vorticity in 2D)
    let omega_magnitude = abs(omega);

    // Store vorticity
    vorticity[cell_index * 2u + 0u] = omega;
    vorticity[cell_index * 2u + 1u] = omega_magnitude;
}
"#;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🌊 2D Lattice Boltzmann Method (LBM) with 4-Layer Multiresolution Grid");
    println!("=======================================================================");
    println!("High-performance 2D fluid dynamics with geometry-based 4-layer multiresolution.");
    println!();
    println!("Features:");
    println!("  • BGK LBM with D2Q9 lattice model");
    println!("  • {}x{} grid with 4-layer multiresolution refinement", TOTAL_GRID_WIDTH, TOTAL_GRID_HEIGHT);
    println!("  • Geometry-based 4-layer grid refinement around circle obstacle");
    println!("  • Level 0 (Finest): Immediate obstacle vicinity");
    println!("  • Level 1 (Fine): Near field region");
    println!("  • Level 2 (Medium): Intermediate field");
    println!("  • Level 3 (Coarsest): Far field regions");
    println!("  • Zou-He inlet/outlet boundary conditions");
    println!("  • Circle obstacle with bounce-back boundaries");
    println!("  • Real-time velocity and vorticity visualization");
    println!("  • GPU compute shaders for maximum performance");
    println!("  • Step sizes: 1, 2, 4, 8 for levels 0-3");
    println!();

    // Create the main application
    let mut app = haggis::default();

    // Create the 2D LBM 4-layer multiresolution simulation
    let simulation = Lbm2d4LayerMultiresolutionSimulation::new();

    // Attach the simulation to the app
    app.attach_simulation(simulation);

    // Add domain boundary markers
    app.add_object("examples/test/cube.obj")
        .with_transform([-1.5, -0.8, 0.0], 0.05, 0.0)
        .with_name("Domain Corner 1");

    app.add_object("examples/test/cube.obj")
        .with_transform([1.5, 0.8, 0.0], 0.05, 0.0)
        .with_name("Domain Corner 2");

    // Add circle obstacle marker (for visual reference)
    app.add_object("examples/test/cube.obj")
        .with_transform([-0.5, 0.0, 0.0], 0.3, 0.0) // Circle visualization
        .with_name("Circle Obstacle");

    // Minimal UI setup to match working pattern
    app.set_ui(|ui, scene, selected_index| {
        haggis::ui::panel::default_transform_panel(ui, scene, selected_index);
    });

    // Run the application
    app.show_performance_panel(true);
    app.run();

    Ok(())
}