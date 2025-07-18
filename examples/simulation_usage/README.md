# Haggis Simulation Usage Examples

This directory contains comprehensive examples demonstrating how to use the Haggis simulation framework at different levels of abstraction.

## Overview

The Haggis simulation framework provides three distinct API levels:

1. **High-Level API** - Simple, declarative interfaces for common simulation tasks
2. **Mid-Level API** - Balanced control with helpful abstractions
3. **Low-Level API** - Direct access to GPU resources for maximum performance

## Examples Structure

### High-Level API Examples (`high_level/`)

**Perfect for beginners** - These examples show how to create simulations with minimal code:

- **`simple_particles.rs`** - Basic particle system in ~30 lines
  ```bash
  cargo run --example simple_particles
  ```
  Demonstrates: Builder pattern, automatic resource management, basic physics

- **`physics_demo.rs`** - Bouncing balls with gravity and collisions
  ```bash
  cargo run --example physics_demo
  ```
  Demonstrates: Multiple force types, boundary constraints, concurrent simulations

- **`flocking_simulation.rs`** - Boids/flocking behavior
  ```bash
  cargo run --example flocking_simulation
  ```
  Demonstrates: Emergent behavior, complex force interactions, multiple particle groups

### Mid-Level API Examples (`mid_level/`)

**For intermediate users** - These examples show more control while maintaining convenience:

- **`custom_forces.rs`** - Custom force implementations with manual control
  ```bash
  cargo run --example custom_forces
  ```
  Demonstrates: Custom simulation logic, time-varying forces, ManagedSimulation wrapper

- **`hybrid_cpu_gpu.rs`** - Intelligent CPU/GPU switching
  ```bash
  cargo run --example hybrid_cpu_gpu
  ```
  Demonstrates: Adaptive performance, resource optimization, performance monitoring

- **`performance_comparison.rs`** - Side-by-side CPU vs GPU benchmarking
  ```bash
  cargo run --example performance_comparison
  ```
  Demonstrates: Performance analysis, scalability testing, bottleneck identification

### Low-Level API Examples (`low_level/`)

**For expert users** - These examples demonstrate maximum control and performance:

- **`custom_compute_shader.rs`** - Custom WGSL compute shaders
  ```bash
  cargo run --example custom_compute_shader
  ```
  Demonstrates: Custom GPU kernels, direct buffer access, advanced algorithms

- **`manual_buffer_management.rs`** - Direct wgpu buffer operations
  ```bash
  cargo run --example manual_buffer_management
  ```
  Demonstrates: Buffer pooling, memory optimization, async transfers

- **`advanced_rendering.rs`** - Custom rendering pipeline integration
  ```bash
  cargo run --example advanced_rendering
  ```
  Demonstrates: Custom shaders, instanced rendering, simulation-to-rendering data flow

## Key Features Demonstrated

### High-Level API Features
- ✅ **Zero Boilerplate**: Get started with just a few lines of code
- ✅ **Builder Pattern**: Fluent, readable configuration
- ✅ **Automatic Resource Management**: No manual GPU setup required
- ✅ **Sensible Defaults**: Works out-of-the-box
- ✅ **Auto CPU/GPU Switching**: Optimal performance automatically

### Mid-Level API Features
- ✅ **Balanced Control**: More flexibility than high-level, less complexity than low-level
- ✅ **Performance Monitoring**: Built-in profiling and metrics
- ✅ **Custom Logic**: Extend base functionality with custom code
- ✅ **Resource Helpers**: Simplified GPU resource management
- ✅ **Hybrid Execution**: Intelligent CPU/GPU switching

### Low-Level API Features
- ✅ **Maximum Performance**: Direct GPU access for experts
- ✅ **Custom Compute Shaders**: Write specialized WGSL kernels
- ✅ **Manual Memory Management**: Fine-grained control over buffers
- ✅ **Custom Rendering**: Direct integration with rendering pipeline
- ✅ **Zero Overhead**: No abstraction penalties

## Getting Started

### For Beginners
Start with the high-level examples:
```bash
cargo run --example simple_particles
cargo run --example physics_demo
```

### For Intermediate Users
Explore the mid-level examples:
```bash
cargo run --example custom_forces
cargo run --example hybrid_cpu_gpu
```

### For Expert Users
Dive into the low-level examples:
```bash
cargo run --example custom_compute_shader
cargo run --example manual_buffer_management
```

## API Progression

The examples are designed to show a natural progression:

1. **Start Simple**: High-level API for quick prototyping
2. **Add Control**: Mid-level API when you need more flexibility
3. **Optimize**: Low-level API for maximum performance

## Common Patterns

### High-Level Pattern
```rust
let particles = ParticleSystem::new()
    .with_count(1000)
    .with_gravity([0.0, 0.0, -9.8])
    .with_ground(0.0)
    .build();
```

### Mid-Level Pattern
```rust
let managed_sim = ManagedSimulation::new(custom_sim)
    .with_debug(true)
    .with_profiling(true);
```

### Low-Level Pattern
```rust
let mut context = ComputeContext::new(device, queue);
context.create_buffer("particles", &data, BufferUsages::STORAGE)?;
context.create_compute_pipeline("update", &shader, "main")?;
```

## Performance Guidance

- **< 1,000 particles**: High-level API is perfect
- **1,000 - 10,000 particles**: Consider mid-level API for optimization
- **> 10,000 particles**: Low-level API for maximum performance
- **Custom algorithms**: Low-level API for specialized compute shaders

## Building and Running

All examples are configured in `Cargo.toml` and can be run with:
```bash
cargo run --example <example_name>
```

## Requirements

- Rust 1.70+
- wgpu-compatible GPU (most modern graphics cards)
- ~100MB RAM for larger examples

## Troubleshooting

### Common Issues

1. **GPU not found**: Ensure you have wgpu-compatible drivers
2. **Out of memory**: Reduce particle count in examples
3. **Compilation errors**: Check Rust version (1.70+ required)

### Performance Tips

- Use `--release` flag for performance testing
- Monitor GPU memory usage with large particle counts
- Consider particle count limits for your hardware

## Contributing

These examples serve as the primary documentation for the Haggis framework. When adding new features:

1. Add corresponding examples at appropriate API levels
2. Update this README with new examples
3. Ensure examples compile and run correctly
4. Include performance characteristics and use cases

## License

These examples are part of the Haggis project and follow the same license terms.