# Haggis Simulation Usage Examples - CPU vs GPU Comparison Framework

This directory contains comprehensive examples demonstrating the Haggis simulation framework with a focus on **educational CPU vs GPU performance comparison**. Each complexity level provides both CPU and GPU implementations to help users understand the trade-offs between different processing approaches.

## Overview

The Haggis simulation framework provides **two distinct API levels** with **CPU vs GPU comparison pairs**:

1. **High-Level API** - Simple, declarative interfaces perfect for learning
   - `cpu/` - Single-threaded CPU implementations with performance monitoring
   - `gpu/` - GPU compute shader implementations with occupancy analysis

2. **Low-Level API** - Direct resource control for advanced optimization
   - `cpu/` - Manual memory management with comprehensive profiling  
   - `gpu/` - Custom compute shaders with GPU performance analysis

## Educational Framework Goals

- **Performance Comparison**: Direct side-by-side CPU vs GPU performance analysis
- **Learning Resource**: Understand when to use CPU vs GPU for particle simulation
- **Optimization Techniques**: Learn both CPU and GPU-specific optimization strategies
- **Scalability Analysis**: Observe how performance scales with particle count
- **Architecture Understanding**: Gain insights into parallel vs sequential processing

## Examples Structure

### High-Level API Examples (`high_level/`)

**Perfect for beginners** - These examples provide direct CPU vs GPU comparison with identical functionality:

#### CPU Implementation (`high_level/cpu/`)
- **`simple_particles.rs`** - CPU-based particle system with performance monitoring
  ```bash
  cargo run --example simple_particles_cpu
  ```
  **Features**: Single-threaded CPU physics, real-time performance metrics, educational framework
  
  **Learning Focus**: CPU sequential processing, cache efficiency, single-core optimization

#### GPU Implementation (`high_level/gpu/`)
- **`simple_particles.rs`** - GPU-based particle system with compute shader analysis  
  ```bash
  cargo run --example simple_particles_gpu
  ```
  **Features**: Parallel GPU compute, workgroup optimization, occupancy monitoring
  
  **Learning Focus**: GPU parallel processing, memory bandwidth, compute shader concepts

**Comparison Value**: Same particle physics logic, different execution models - perfect for understanding CPU vs GPU trade-offs

### Low-Level API Examples (`low_level/`)

**For advanced users** - These examples demonstrate maximum control and comprehensive performance analysis:

#### CPU Implementation (`low_level/cpu/`)
- **`performance_comparison.rs`** - Advanced CPU optimization with comprehensive profiling
  ```bash
  cargo run --example performance_comparison_cpu
  ```
  **Features**: Manual memory management, custom force implementations, multi-threading support, bottleneck identification
  
  **Learning Focus**: CPU optimization techniques, memory access patterns, cache-friendly algorithms, threading strategies

#### GPU Implementation (`low_level/gpu/`)
- **`performance_comparison.rs`** - Custom compute shaders with GPU performance analysis
  ```bash
  cargo run --example performance_comparison_gpu
  ```
  **Features**: Custom WGSL shaders, GPU memory hierarchy optimization, workgroup tuning, occupancy analysis
  
  **Learning Focus**: GPU compute pipeline, memory coalescing, shared memory usage, workgroup optimization

**Advanced Comparison**: Incorporates the sophisticated performance analysis from the previous mid-level API, now split into CPU and GPU specializations

#### Additional GPU Examples (`low_level/gpu/`)
These examples showcase specialized GPU techniques:

- **`custom_compute_shader.rs`** - Advanced WGSL compute shader techniques
  ```bash
  cargo run --example custom_compute_shader
  ```
  **Features**: Specialized GPU kernels, advanced algorithms, multi-pass rendering

- **`manual_buffer_management.rs`** - Direct GPU buffer operations
  ```bash
  cargo run --example manual_buffer_management
  ```
  **Features**: Buffer pooling, memory optimization, async transfers

- **`advanced_rendering.rs`** - Custom rendering pipeline integration
  ```bash
  cargo run --example advanced_rendering
  ```
  **Features**: Custom shaders, instanced rendering, simulation-to-rendering data flow

## CPU vs GPU Comparison Features

### High-Level API - Educational Features
#### CPU Features
- ✅ **Sequential Processing**: Clear single-threaded execution model
- ✅ **Performance Monitoring**: Real-time FPS and timing analysis
- ✅ **Cache-Friendly Design**: Educational memory access patterns
- ✅ **Debugging Support**: Easy to step through and understand
- ✅ **Minimal Setup**: Works immediately without GPU requirements

#### GPU Features  
- ✅ **Parallel Processing**: Thousands of particles computed simultaneously
- ✅ **Compute Shaders**: Learn GPU programming concepts
- ✅ **Occupancy Analysis**: Understand GPU resource utilization
- ✅ **Memory Hierarchy**: Global, shared, and local memory usage
- ✅ **Workgroup Optimization**: Tune for different GPU architectures

### Low-Level API - Advanced Features
#### CPU Advanced Features
- ✅ **Manual Memory Management**: Custom allocation strategies for performance
- ✅ **Multi-threading Support**: Explore parallel CPU execution
- ✅ **Custom Force Models**: Implement sophisticated algorithms
- ✅ **Comprehensive Profiling**: Detailed bottleneck identification
- ✅ **Optimization Analysis**: Automatic performance suggestions

#### GPU Advanced Features
- ✅ **Custom WGSL Shaders**: Write specialized compute kernels
- ✅ **GPU Memory Optimization**: Efficient buffer management
- ✅ **Workgroup Tuning**: Optimize for specific hardware
- ✅ **Performance Counters**: Hardware-level profiling
- ✅ **Occupancy Optimization**: Maximize GPU resource usage

## Getting Started with CPU vs GPU Comparison

### For Beginners - Learn the Fundamentals
Start with high-level CPU vs GPU comparison:
```bash
# Run CPU implementation
cargo run --example simple_particles_cpu

# Run GPU implementation (in separate window)
cargo run --example simple_particles_gpu
```
**Learning Goal**: Understand basic differences between sequential and parallel processing

### For Advanced Users - Deep Performance Analysis
Explore low-level optimization techniques:
```bash
# Advanced CPU optimization
cargo run --example performance_comparison_cpu

# Advanced GPU optimization  
cargo run --example performance_comparison_gpu
```
**Learning Goal**: Master architecture-specific optimization techniques

### Recommended Learning Path
1. **Start with High-Level CPU**: Understand basic particle physics
2. **Compare with High-Level GPU**: See parallel processing benefits
3. **Explore Low-Level CPU**: Learn manual optimization techniques  
4. **Master Low-Level GPU**: Understand compute shader programming

## Educational Framework Progression

The examples are designed to show a natural learning progression:

1. **Learn Fundamentals**: High-level API to understand basic concepts
   - CPU: Sequential processing, single-threaded execution
   - GPU: Parallel processing, compute shader basics

2. **Master Optimization**: Low-level API for advanced techniques
   - CPU: Memory management, threading, cache optimization
   - GPU: Workgroup tuning, memory hierarchy, custom shaders

3. **Compare Architectures**: Understand when to use each approach
   - Small particle counts: CPU often more efficient
   - Large particle counts: GPU provides massive speedup
   - Complex algorithms: May favor CPU for flexibility
   - Simple operations: GPU excels with parallel execution

## Common Patterns

### CPU Pattern (High-Level)
```rust
let cpu_sim = SimpleParticleSystemCPU::new();
// Features: Sequential processing, performance monitoring
haggis.attach_simulation(cpu_sim);
```

### GPU Pattern (High-Level)  
```rust
let gpu_sim = SimpleParticleSystemGPU::new();
// Features: Parallel processing, compute shaders
haggis.attach_simulation(gpu_sim);
```

### CPU Pattern (Low-Level)
```rust
let cpu_sim = LowLevelCPUSimulation::new()
    .with_threading(true)
    .with_profiling(true);
// Features: Manual optimization, custom forces
```

### GPU Pattern (Low-Level)
```rust
let gpu_sim = LowLevelGPUSimulation::new()
    .with_workgroup_size(64)
    .with_shared_memory(true);
// Features: Custom shaders, occupancy tuning
```

## CPU vs GPU Performance Guidance

### When to Choose CPU
- **< 500 particles**: CPU overhead is minimal, GPU setup cost dominates
- **Complex algorithms**: CPU provides more flexible branching and logic
- **Debug/development**: Easier to step through and understand execution
- **Limited GPU memory**: CPU can use system RAM more flexibly
- **Quick prototyping**: Faster to implement and iterate

### When to Choose GPU  
- **> 1,000 particles**: GPU parallel processing shows clear benefits
- **Simple operations**: Uniform computations across all particles
- **High throughput**: Need to process many particles per frame
- **Real-time applications**: GPU can maintain consistent high FPS
- **Scalable solutions**: Need to handle variable particle counts efficiently

### Crossover Analysis
The examples help identify the crossover point where GPU becomes more efficient than CPU for your specific use case and hardware configuration.

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