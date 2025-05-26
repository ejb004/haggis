# Haggis Engine File Layout & Structure

## File Structure

```
src/
├── lib.rs
├── app.rs
├── wgpu_utils/
│   ├── mod.rs
│   ├── pipeline.rs
│   ├── buffer.rs      // Your existing buffer convenience functions
│   ├── bind_group.rs  // Your existing bind group convenience functions
│   └── texture.rs
├── graphics/
│   ├── mod.rs
│   └── renderer.rs
├── scene/
│   ├── mod.rs
│   ├── scene.rs
│   ├── camera.rs
│   └── transform.rs
├── object/
│   ├── mod.rs
│   ├── object.rs
│   ├── material.rs
│   ├── mesh.rs
│   └── loader.rs
├── simulation/
│   ├── mod.rs
│   ├── cpu/
│   │   ├── mod.rs
│   │   ├── physics.rs
│   │   └── particles.rs
│   └── gpu/
│       ├── mod.rs
│       ├── compute.rs
│       └── shaders/
│           ├── physics.wgsl
│           └── particles.wgsl
├── utils/
│   ├── mod.rs
│   ├── math.rs
│   └── time.rs
└── shaders/
    ├── vertex.wgsl
    ├── fragment.wgsl
    └── compute/
        └── simulation.wgsl
```

## Core Structs and Their Contents

### `HaggisApp` (app.rs)

```rust
pub struct HaggisApp {
    pub event_loop: Option<EventLoop<()>>,
    pub window: Window,
    pub renderer: Renderer,
    pub scene: Scene,
    pub simulation_manager: SimulationManager,
    pub last_frame_time: Instant,
    pub target_fps: Option<u32>,
}

impl HaggisApp {
    pub async fn new() -> Self { /* ... */ }
    pub fn add_object(&mut self, object: Object) { /* ... */ }
    pub fn run(self) { /* ... */ }
}
```

### `Renderer` (graphics/renderer.rs)

```rust
pub struct Renderer {
    pub surface: wgpu::Surface,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub size: winit::dpi::PhysicalSize<u32>,
    pub depth_texture: Texture,
    pub pipeline_manager: PipelineManager,
}
```

### `PipelineManager` (wgpu_utils/pipeline.rs)

```rust
pub struct PipelineManager {
    pub render_pipelines: HashMap<String, wgpu::RenderPipeline>,
    pub compute_pipelines: HashMap<String, wgpu::ComputePipeline>,
    pub bind_group_layouts: HashMap<String, wgpu::BindGroupLayout>,
    device: Arc<wgpu::Device>, // Shared reference
    surface_format: wgpu::TextureFormat,
}

impl PipelineManager {
    // Uses your existing wgpu_utils convenience functions internally
    pub fn create_render_pipeline(&mut self, desc: RenderPipelineDescriptor) -> &wgpu::RenderPipeline { /* ... */ }
    pub fn create_compute_pipeline(&mut self, desc: ComputePipelineDescriptor) -> &wgpu::ComputePipeline { /* ... */ }
}
```

### `Scene` (scene/scene.rs)

```rust
pub struct Scene {
    pub objects: Vec<Object>,
    pub camera: Camera,
    pub lights: Vec<Light>,
    pub environment: Environment,
    pub physics_world: Option<PhysicsWorld>, // For CPU simulation
}
```

### `Object` (object/object.rs)

```rust
pub struct Object {
    pub id: u32,
    pub mesh: Mesh,
    pub material: Material,
    pub transform: Transform,
    pub physics_body: Option<PhysicsBody>, // For simulation
    pub simulation_data: Option<SimulationData>,
}
```

### `Mesh` (object/mesh.rs)

```rust
pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub vertex_buffer: Option<wgpu::Buffer>,
    pub index_buffer: Option<wgpu::Buffer>,
    pub vertex_count: u32,
    pub index_count: u32,
}

impl Mesh {
    // Uses your wgpu_utils buffer convenience functions
    pub fn create_gpu_buffers(&mut self, device: &wgpu::Device) {
        // Uses wgpu_utils::buffer::create_vertex_buffer()
        // Uses wgpu_utils::buffer::create_index_buffer()
    }
}
```

### `Material` (object/material.rs)

```rust
pub struct Material {
    pub name: String,
    pub albedo: [f32; 4],
    pub metallic: f32,
    pub roughness: f32,
    pub normal_map: Option<Texture>,
    pub albedo_texture: Option<Texture>,
    pub pipeline_id: String, // References pipeline in PipelineManager
    pub bind_group: Option<wgpu::BindGroup>,
    pub uniform_buffer: Option<wgpu::Buffer>,
}

impl Material {
    // Uses your wgpu_utils for buffer and bind group creation
    pub fn create_gpu_resources(&mut self, device: &wgpu::Device, layout: &wgpu::BindGroupLayout) {
        // Uses wgpu_utils::buffer::create_uniform_buffer()
        // Uses wgpu_utils::bind_group::create_bind_group()
    }
}
```

### `Camera` (scene/camera.rs)

```rust
pub struct Camera {
    pub position: Vec3,
    pub target: Vec3,
    pub up: Vec3,
    pub fovy: f32,
    pub aspect: f32,
    pub znear: f32,
    pub zfar: f32,
    pub view_matrix: Mat4,
    pub projection_matrix: Mat4,
    pub uniform_buffer: Option<wgpu::Buffer>,
    pub bind_group: Option<wgpu::BindGroup>,
}

impl Camera {
    // Uses wgpu_utils for GPU resource creation
    pub fn create_gpu_resources(&mut self, device: &wgpu::Device, layout: &wgpu::BindGroupLayout) {
        // Uses wgpu_utils::buffer::create_uniform_buffer()
        // Uses wgpu_utils::bind_group::create_bind_group()
    }
}
```

### `Transform` (scene/transform.rs)

```rust
pub struct Transform {
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
    pub matrix: Mat4, // Cached transform matrix
    pub uniform_buffer: Option<wgpu::Buffer>,
    pub needs_update: bool,
}

impl Transform {
    // Uses wgpu_utils for buffer operations
    pub fn update_gpu_buffer(&mut self, queue: &wgpu::Queue) {
        // Uses wgpu_utils::buffer::write_buffer()
    }
}
```

## Simulation System

### `SimulationManager` (simulation/mod.rs)

```rust
pub struct SimulationManager {
    pub cpu_simulator: Option<CpuSimulator>,
    pub gpu_simulator: Option<GpuSimulator>,
    pub active_mode: SimulationMode,
    pub timestep: f32,
    pub paused: bool,
}

pub enum SimulationMode {
    None,
    Cpu,
    Gpu,
    Hybrid,
}
```

### `CpuSimulator` (simulation/cpu/mod.rs)

```rust
pub struct CpuSimulator {
    pub physics_engine: PhysicsEngine,
    pub particle_systems: Vec<ParticleSystem>,
    pub thread_pool: ThreadPool,
}
```

### `GpuSimulator` (simulation/gpu/mod.rs)

```rust
pub struct GpuSimulator {
    pub compute_pipelines: HashMap<String, wgpu::ComputePipeline>,
    pub simulation_buffers: HashMap<String, wgpu::Buffer>,
    pub bind_groups: HashMap<String, wgpu::BindGroup>,
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
}

impl GpuSimulator {
    // Uses wgpu_utils for compute pipeline and buffer creation
    pub fn create_simulation_pipeline(&mut self, shader: &str, entry_point: &str) {
        // Uses wgpu_utils::pipeline::create_compute_pipeline()
    }

    pub fn create_simulation_buffers(&mut self, data: &[f32]) {
        // Uses wgpu_utils::buffer::create_storage_buffer()
    }
}
```

## Resource Management

### `Vertex` (object/mesh.rs)

```rust
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub tex_coords: [f32; 2],
    pub tangent: [f32; 3],
}
```

### `Texture` (wgpu_utils/texture.rs)

```rust
pub struct Texture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    pub size: wgpu::Extent3d,
}

// Convenience functions that use your existing wgpu_utils
impl Texture {
    pub fn from_bytes(device: &wgpu::Device, queue: &wgpu::Queue, bytes: &[u8], label: &str) -> Self { /* ... */ }
    pub fn create_depth_texture(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration, label: &str) -> Self { /* ... */ }
}
```

## Key Design Considerations

### wgpu_utils Integration

- All GPU resource creation goes through your existing `wgpu_utils` convenience functions
- `PipelineManager` wraps your pipeline creation utilities with caching and management
- Materials, meshes, cameras, and transforms all use `wgpu_utils` for GPU operations
- Simulation systems leverage `wgpu_utils` for compute pipeline and buffer management

### Device Sharing Strategy

- Use `Arc<wgpu::Device>` and `Arc<wgpu::Queue>` for shared access
- `PipelineManager` holds device reference for pipeline creation
- Objects store buffer references, not device references
- Initialization order: Device → PipelineManager → Objects

### Buffer Management

- Lazy buffer creation: buffers created when object is added to renderer
- `needs_gpu_update` flags on objects for efficient updates
- Separate staging buffers for dynamic data

### Pipeline Organization

- Material types map to specific pipelines
- Pipeline caching by material signature
- Compute pipelines separate from render pipelines

### Simulation Integration

- Objects can have both CPU and GPU simulation data
- Simulation results update transform matrices
- Flexible switching between CPU/GPU simulation modes

## Usage Pattern (Your Desired API)

```rust
// Simple, batteries-included API
let mut haggis = haggis::new().await; // Creates HaggisApp with everything set up
let object = haggis::Object::from_file("path.obj") // Static method
    .with_material(haggis::Material::default()); // Builder pattern
haggis.add_object(object); // Handles GPU resource creation
haggis.run(); // Consumes event loop, starts render loop
```

## Main Exports (lib.rs)

```rust
// Re-export main app and types
pub use app::HaggisApp;
pub use object::{Object, Material, Mesh};
pub use scene::{Scene, Camera, Transform};
pub use simulation::{SimulationManager, SimulationMode};

// Convenience function for your desired API
pub async fn new() -> HaggisApp {
    HaggisApp::new().await
}

// For users who want sync version
pub fn default() -> HaggisApp {
    pollster::block_on(HaggisApp::new())
}
```
