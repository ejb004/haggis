# Haggis 3D Engine Technical Documentation

## 1. Framework Overview

Haggis is a GPU-accelerated 3D rendering and simulation engine built in Rust using wgpu for modern graphics and winit for cross-platform windowing. It provides both high-level abstractions for beginners and low-level GPU access for advanced users.

**Core Philosophy**: Bridge the gap between simple 3D visualization and complex simulations by providing layered APIs that scale from beginner-friendly to expert-level control.

**Coordinate System**: Z-up (X=right, Y=forward, Z=up)

## 2. Basic Usage - Getting Started

### 2.1 Minimal Example
```rust
use haggis;

fn main() {
    let mut app = haggis::default();
    app.add_object("model.obj").with_transform([0.0, 0.0, 0.0], 1.0, 0.0);
    app.run();
}
```

### 2.2 Conway's Game of Life (Beginner Simulation)
```rust
use haggis::simulation::BaseSimulation;

struct ConwaySimulation {
    grid: Vec<Vec<bool>>,
    speed: f32,
}

impl Simulation for ConwaySimulation {
    fn update(&mut self, dt: f32, scene: &mut Scene, device: Option<&Device>, queue: Option<&Queue>) {
        // CPU-based grid update logic
        self.update_grid();
        self.update_visualization(scene);
    }
    
    fn name(&self) -> &str { "Conway's Game of Life" }
    
    fn render_ui(&mut self, ui: &Ui, scene: &mut Scene) {
        ui.slider("Speed", 0.1, 2.0, &mut self.speed);
    }
}
```

### 2.3 Scene Building
```rust
let mut app = haggis::default();

// Add objects with materials
app.add_object("cube.obj")
    .with_material("red_plastic")
    .with_transform([2.0, 0.0, 0.0], 1.5, 45.0)
    .with_name("rotating_cube");

// Create materials
scene.add_material_rgb("red_plastic", 0.8, 0.2, 0.2, 0.0, 0.3);
```

## 3. Advanced Features

### 3.1 Simulation System Architecture

The framework provides three abstraction levels:

#### High-Level API (Beginners)
- **ParticleSystem**: Declarative particle physics with builder pattern
- **ForceField**: Gravity, wind, springs with automatic application
- **Constraints**: Boundaries, collision detection
- **Auto GPU/CPU**: Framework chooses optimal execution

```rust
let particles = ParticleSystem::new()
    .with_count(10000)
    .with_gravity([0.0, 0.0, -9.8])
    .with_bounds([-10.0, 10.0], [-10.0, 10.0], [0.0, 20.0])
    .build();
```

#### Mid-Level API (Intermediate)
- **BaseSimulation**: Template for custom simulations
- **CPU/GPU Toggle**: Runtime switching between execution modes
- **Visualization Integration**: Built-in 2D/3D data visualization
- **Performance Monitoring**: Built-in profiling and metrics

#### Low-Level API (Experts)  
- **ComputeContext**: Direct wgpu buffer and pipeline management
- **Custom Compute Shaders**: Full GPU programming control
- **Memory Management**: Manual allocation and optimization
- **Multi-pass Rendering**: Complex visualization pipelines

### 3.2 GPU Compute Integration

**Compute Shader System**:
- Ping-pong buffering for iterative algorithms
- Automatic workgroup size calculation
- Pipeline caching and reuse
- Integration with rendering pipeline

**Example: Fluid Dynamics GPU Implementation**
```rust
impl Simulation for FluidSim {
    fn update(&mut self, dt: f32, scene: &mut Scene, device: Option<&Device>, queue: Option<&Queue>) {
        if let (Some(device), Some(queue)) = (device, queue) {
            self.gpu_fluid_step(device, queue, dt);
            self.update_visualization(scene);
        }
    }
}
```

### 3.3 Visualization System

**Modular Components**:
- **CutPlane2D**: Cross-sections of 3D data
- **Volume Rendering**: Direct volume visualization  
- **Particle Rendering**: Instanced particle display
- **Field Visualization**: Vector/scalar field display

**Integration Example**:
```rust
let mut viz_manager = VisualizationManager::new();
viz_manager.add_component("fluid_density", Box::new(cut_plane));
app.attach_visualization(viz_manager);
```

### 3.4 Complex Use Cases

**Fluid Dynamics**: Navier-Stokes solving with GPU compute shaders, real-time pressure/velocity field visualization

**Educational Simulations**: Interactive physics demos with step-through controls, parameter adjustment UI, real-time graph plotting

**Ray Tracing Extensions**: Custom shaders for lighting models, BVH acceleration structures, progressive refinement

**Note**: Animation systems are not currently a framework focus - emphasis is on simulation and real-time visualization.

## 4. Code-Level Architecture

### 4.1 Module Structure

**Core Architecture Layers:**

1. **`app.rs`**: Application lifecycle using winit's `ApplicationHandler`
   - Event processing (window, device, UI)
   - Main update/render loop coordination
   - State management between systems

2. **`gfx/`**: Graphics rendering system
   - **`rendering/`**: PBR pipeline, shadow mapping, GPU resource management
   - **`scene/`**: Object hierarchy, materials, vertex data
   - **`camera/`**: Orbit camera with smooth interpolation
   - **`resources/`**: Texture loading, material system, GPU buffer management

3. **`simulation/`**: Layered simulation framework
   - **`traits.rs`**: Core `Simulation` trait defining update/render interface
   - **`high_level.rs`**: Builder pattern APIs with automatic resource management
   - **`low_level.rs`**: Direct wgpu access for compute shaders
   - **`manager.rs`**: Simulation lifecycle and execution coordination

4. **`visualization/`**: Modular data visualization
   - **`traits.rs`**: `VisualizationComponent` trait for pluggable visualizers
   - **`rendering/`**: Specialized renderers for 2D/3D data
   - **`ui/`**: Control panels for visualization parameters

5. **`wgpu_utils/`**: GPU abstraction utilities
   - **`binding_builder.rs`**: Fluent API for bind group creation
   - **`uniform_buffer.rs`**: Simplified buffer management
   - **`binding_types.rs`**: Common binding patterns

### 4.2 Key Abstractions

**Simulation Trait System:**
```rust
pub trait Simulation {
    fn update(&mut self, dt: f32, scene: &mut Scene, device: Option<&Device>, queue: Option<&Queue>);
    fn name(&self) -> &str;
    fn render_ui(&mut self, ui: &Ui, scene: &mut Scene);
}
```

**GPU/CPU Integration:**
- Optional device/queue parameters enable GPU access
- Scene provides unified interface for visual updates
- Automatic fallback to CPU-only operation

**Resource Management Pattern:**
```rust
// High-level: Automatic management
let particles = ParticleSystem::new().with_count(1000).build();

// Low-level: Manual control
let context = ComputeContext::new(device, queue);
context.create_buffer("positions", size, BufferUsages::STORAGE);
```

**Scene Graph Architecture:**
- **Object**: Transform, material reference, mesh data
- **Material**: PBR parameters, GPU buffer representation
- **Scene**: Manages object collection, material registry, GPU synchronization

### 4.3 GPU Integration Patterns

**Compute Shader Integration:**
- Ping-pong buffering for iterative algorithms
- Automatic workgroup size calculation based on data size
- Pipeline caching for performance

**Rendering Pipeline:**
- Forward rendering with depth pre-pass
- Shadow mapping with Gaussian blur post-processing
- Unified lighting model across all objects

**Memory Management:**
- Shared `Arc<Device>` and `Arc<Queue>` for resource sharing
- Lazy GPU buffer creation on first use
- Automatic buffer resizing and reallocation

## 5. Refactoring Suggestions

### 5.1 Beginner–Advanced Balance

**Current Issues:**
- High-level APIs in `simulation/high_level.rs` not fully implemented
- Missing tutorial progression from basic to advanced concepts  
- Examples jump between complexity levels without clear transitions

**Recommended Changes:**

1. **Implement Complete High-Level API:**
   ```rust
   // Fill out ParticleSystem, ForceField, Constraint implementations
   // Add automatic GPU/CPU selection based on particle count
   // Provide sensible defaults for common physics scenarios
   ```

2. **Create Progressive Example Series:**
   - **Level 1**: Static scene with basic object placement
   - **Level 2**: Simple animation with transform updates  
   - **Level 3**: CPU-based Conway's Game of Life
   - **Level 4**: GPU compute shader introduction
   - **Level 5**: Complex fluid simulation with visualization

3. **Standardize API Conventions:**
   ```rust
   // Consistent builder pattern across all systems
   app.simulation().particles().count(1000).gravity([0,0,-9.8]).build();
   app.visualization().cut_plane().axis('z').position(0.0).build();
   ```

### 5.2 Documentation and Examples

**Issues:**
- Sparse inline documentation for complex GPU code
- Missing migration guides between abstraction levels
- No performance benchmarking examples

**Solutions:**

1. **API Documentation Standards:**
   - All public functions require usage examples
   - GPU concepts explained with diagrams in doc comments
   - Performance characteristics documented for each API level

2. **Interactive Examples:**
   - Conway's Game → N-body simulation → Fluid dynamics progression
   - Each example shows equivalent CPU/GPU implementations
   - Benchmark comparison utilities built into framework

### 5.3 Module Organization

**Current Problems:**
- `simulation/` contains both examples and core APIs
- GPU utilities scattered across multiple modules
- Visualization tightly coupled to simulation

**Proposed Structure:**
```
src/
├── core/           # App, event handling, lifecycle
├── graphics/       # Rendering, materials, scene
├── compute/        # GPU compute abstractions
├── simulation/     # Simulation traits and managers
├── visualization/  # Data visualization components  
└── examples/       # Progressive tutorial examples
```

## 6. Priority Roadmap

### Priority 1 (Critical - Foundation)

1. **Complete High-Level Simulation API** (2-3 weeks)
   - Implement `ParticleSystem`, `ForceField`, `Constraint` classes
   - Add automatic GPU/CPU selection logic
   - **Impact**: Makes framework accessible to beginners

2. **Standardize Builder Patterns** (1 week)  
   - Consistent API across simulation, visualization, scene building
   - Fluent method chaining for all major systems
   - **Impact**: Reduces learning curve, improves usability

3. **GPU Compute Abstraction Layer** (2 weeks)
   - Simplify compute shader integration
   - Automatic buffer management for common patterns
   - **Impact**: Makes GPU programming accessible to intermediate users

### Priority 2 (Important - Usability)

4. **Progressive Tutorial Series** (2 weeks)
   - 5-level example progression from basic to advanced
   - Each level builds on previous concepts
   - **Impact**: Clear learning path for all skill levels

5. **Performance Benchmarking Tools** (1 week)
   - Built-in CPU vs GPU comparison utilities
   - Memory usage and timing analysis
   - **Impact**: Helps users choose appropriate abstraction level

6. **Visualization Component System** (1.5 weeks)
   - Decouple visualization from simulation
   - Plugin architecture for custom visualizers
   - **Impact**: Increases framework flexibility and extensibility

### Priority 3 (Enhancement - Advanced Features)

7. **Memory Pool Management** (1.5 weeks)
   - Efficient GPU buffer reuse and allocation strategies
   - Reduce garbage collection pressure
   - **Impact**: Better performance for complex simulations

8. **Multi-pass Rendering Pipeline** (2 weeks)
   - Support for complex lighting models and post-processing
   - Ray tracing integration groundwork
   - **Impact**: Enables advanced graphics techniques

9. **Async Compute Pipeline** (2 weeks)
   - Background GPU computation while rendering
   - Timeline synchronization between compute and graphics
   - **Impact**: Better utilization of GPU resources

### Priority 4 (Polish - Long-term)

10. **WebGPU Support** (3-4 weeks)
    - Cross-platform compatibility including web browsers
    - Feature parity with native implementation
    - **Impact**: Broader platform reach

## 7. Optimization Ideas

### 7.1 CPU/GPU Performance

**GPU Optimizations:**

1. **Buffer Management:**
   - Implement buffer pools to avoid allocation overhead
   - Use persistent mapping for frequently updated data
   - Batch GPU operations to reduce command queue overhead

2. **Compute Shader Efficiency:**
   - Automatic workgroup size tuning based on hardware
   - Shared memory optimization for data-intensive algorithms
   - Asynchronous compute when possible

3. **Memory Layout Optimization:**
   ```rust
   // Structure of Arrays (SoA) for better GPU access patterns
   struct ParticlesSoA {
       positions: Vec<Vector3<f32>>,
       velocities: Vec<Vector3<f32>>,
       // Better cache coherence than Array of Structures
   }
   ```

**CPU Performance:**

1. **Multi-threading Strategy:**
   - Work-stealing for irregular workloads
   - SIMD utilization for vector operations
   - Lock-free data structures where possible

2. **Cache Optimization:**
   - Data locality improvements in scene graph traversal
   - Prefetching strategies for large simulations

### 7.2 Modularization

**Current Issues:**
- Tight coupling between simulation and rendering
- Global state management complexity
- Difficulty testing components in isolation

**Solutions:**

1. **Dependency Injection:**
   ```rust
   pub trait GraphicsBackend {
       fn create_buffer(&self, size: u64) -> BufferHandle;
       fn dispatch_compute(&self, pipeline: &ComputePipeline, groups: [u32; 3]);
   }
   
   // Enables testing with mock backends
   pub struct Simulation<G: GraphicsBackend> {
       graphics: G,
   }
   ```

2. **Event System:**
   - Decouple components through event messaging
   - Enable runtime component addition/removal
   - Support for custom event types in simulations

3. **Plugin Architecture:**
   ```rust
   pub trait HaggisPlugin {
       fn initialize(&self, app: &mut HaggisApp);
       fn update(&self, dt: f32, world: &mut World);
   }
   ```

### 7.3 Simplification

**API Complexity Reduction:**

1. **Smart Defaults:**
   - Auto-detect optimal GPU/CPU execution based on data size
   - Sensible material parameters for common use cases
   - Automatic camera positioning based on scene bounds

2. **Error Handling:**
   - Convert panics to recoverable errors with suggestions
   - Validation with helpful error messages for beginners
   - Fallback strategies for resource allocation failures

3. **Compilation Time:**
   - Reduce template instantiations through type erasure where appropriate
   - Separate debug/release compilation features
   - Incremental compilation optimization

**Memory Management:**
- RAII patterns for all GPU resources
- Automatic cleanup on simulation end
- Reference counting for shared resources (textures, shaders)

**Development Experience:**
- Hot reloading for shaders during development
- Built-in profiler integration
- Debug visualization options for GPU buffers and pipeline state

---

## Summary

Haggis provides a well-architected foundation for GPU-accelerated 3D simulations with clear separation between beginner-friendly high-level APIs and expert-level GPU control. The framework excels at compute shader integration and real-time visualization but needs completion of high-level abstractions and better tutorial progression to fully achieve its goal of serving both beginners and advanced users effectively.

The modular architecture enables extension toward complex applications like fluid dynamics and educational simulations, while the layered API design provides natural upgrade paths as users' requirements become more sophisticated.