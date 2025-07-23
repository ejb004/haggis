# Haggis 🏴󠁧󠁢󠁳󠁣󠁴󠁿

[![Crates.io](https://img.shields.io/crates/v/haggis)](https://crates.io/crates/haggis)
[![Documentation](https://docs.rs/haggis/badge.svg)](https://docs.rs/haggis)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A general-purpose GPU compute and render engine for simulation visualizations built in Rust.

Haggis provides a high-level API for creating real-time 3D simulations with GPU acceleration, featuring both CPU and GPU compute capabilities, modern PBR rendering, and interactive visualizations.

## ✨ Features

- **🎮 Simple API**: Easy-to-use builder pattern for quick prototyping
- **⚡ GPU Acceleration**: High-performance compute shaders for large-scale simulations
- **🎨 Modern Rendering**: PBR (Physically Based Rendering) with shadow mapping
- **📊 Built-in Visualizations**: 2D cut planes, particle systems, and data visualization
- **🎛️ Interactive UI**: ImGui integration for real-time parameter control
- **🔄 Flexible Architecture**: Support for both CPU and GPU simulation backends
- **📐 Z-up Coordinate System**: Industry-standard coordinate system for 3D graphics

## 🚀 Quick Start

Add haggis to your `Cargo.toml`:

```toml
[dependencies]
haggis = "0.1"
```

### Basic Example

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create the application
    let mut app = haggis::default();

    // Add a 3D object
    app.add_object("src/monkey.obj")
        .with_material("gold")
        .with_transform([0.0, 0.0, 0.0], 1.0, 0.0);

    // Create materials
    app.app_state
        .scene
        .add_material_rgb("gold", 1.0, 0.84, 0.0, 1.0, 0.5);

    // Run the application
    app.run();
    Ok(())
}



```

## 📚 Examples

### Core Examples

- **`cargo run --example test`** - Basic 3D object loading and rendering
- **`cargo run --example conways_game_of_life`** - GPU-accelerated Conway's Game of Life
- **`cargo run --example conways_game_of_life_cpu`** - CPU implementation for comparison
- **`cargo run --example three_body`** - N-body gravitational simulation

### MORE Examples

- **`cargo run --example quickstart`** - Comprehensive getting started guide
- High-level API examples in `examples/simulation_usage/high_level/`
- Low-level GPU examples in `examples/simulation_usage/low_level/`

## 🏗️ Architecture

### Core Components

- **`HaggisApp`**: Main application entry point with simple builder API
- **`Scene`**: 3D scene management with objects, materials, and camera
- **`Simulation`**: Trait for implementing custom simulations (CPU/GPU)
- **`CutPlane2D`**: 2D data visualization component with filtering options
- **`MaterialManager`**: PBR material system with metallic/roughness workflow

### Coordinate System

Haggis uses a **Z-up coordinate system**:

- X-axis: Right
- Y-axis: Forward
- Z-axis: Up

This matches industry standards and provides intuitive 3D object placement.

### Simulation Types

1. **CPU Simulations**: Traditional Rust code with threading support
2. **GPU Simulations**: Compute shaders for high-performance parallel processing
3. **Hybrid**: Combine CPU logic with GPU acceleration where needed

## 🎨 Visualization Features

### 2D Cut Plane Visualization

- **Sharp/Smooth Filtering**: Toggle between pixelated and interpolated rendering
- **Multiple Modes**: Heatmap, grid patterns, and point visualization
- **GPU Buffer Support**: Direct GPU buffer visualization for zero-copy performance
- **Interactive Controls**: Real-time adjustment of position, size, and rendering style

### 3D Rendering

- **PBR Materials**: Physically based rendering with metallic/roughness
- **Shadow Mapping**: Real-time shadow casting and receiving
- **Object Loading**: OBJ file support with automatic material extraction
- **Camera Controls**: Orbit camera with mouse and keyboard input

## 🛠️ Development

### Building from Source

```bash
git clone https://github.com/ejb004/haggis.git
cd haggis
cargo build --release
```

### Running Examples

```bash
# Basic 3D rendering
cargo run --example test

# Conway's Game of Life (GPU)
cargo run --example conways_game_of_life

# Three-body simulation
cargo run --example three_body
```

### Code Formatting

```bash
cargo fmt
cargo clippy
```

## 📋 Requirements

- **Rust**: 1.70.0 or later
- **Graphics**: DirectX 12, Vulkan, Metal, or OpenGL ES 3.0
- **Platform**: Windows, macOS, or Linux

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request. For major changes, please open an issue first to discuss what you would like to change.

### Development Guidelines

1. Follow Rust standard formatting (`cargo fmt`)
2. Ensure all tests pass (`cargo test`)
3. Add examples for new features
4. Update documentation as needed

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🔗 Dependencies

- **[wgpu](https://wgpu.rs/)**: Modern graphics API abstraction
- **[winit](https://github.com/rust-windowing/winit)**: Cross-platform windowing
- **[cgmath](https://github.com/rustgd/cgmath)**: Linear algebra for 3D mathematics
- **[imgui](https://github.com/imgui-rs/imgui-rs)**: Immediate mode GUI
- **[tobj](https://github.com/syoyo/tinyobjloader-rs)**: OBJ file loading

## 🚧 Roadmap

- [ ] Fix shadow map to cover all working area and optimise for caching
- [ ] Additional file format support (glTF, FBX)
- [ ] Advanced particle systems
- [ ] Networking for distributed simulations
- [ ] More built-in simulation examples
- [ ] Performance profiling tools
- [ ] WebAssembly support

---

**Haggis** - _Because every good simulation needs a bit of Scottish engineering_ 🏴󠁧󠁢󠁳󠁣󠁴󠁿
