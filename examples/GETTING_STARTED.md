# 🚀 Getting Started with Haggis

Welcome to the Haggis particle simulation framework! This guide will help you choose the right starting point based on your experience level.

## 📍 Where to Start

### 🌟 **New to Haggis? Start Here!**

**[Quickstart Example](quickstart/README.md)** - `cargo run --example quickstart`
- Perfect first example for beginners
- Demonstrates core framework concepts
- Well-commented, educational code
- Interactive physics controls
- Clear explanations of every step

### 📈 **Learning Progression**

Once you've mastered the quickstart, follow this path:

#### 1️⃣ **Quickstart** ← *You are here*
```bash
cargo run --example quickstart
```
**What you'll learn**: Basic particle creation, simple physics, rendering setup, UI basics

#### 2️⃣ **High-Level Examples** 
```bash
# Compare CPU vs GPU implementations
cargo run --example simple_particles_cpu
cargo run --example simple_particles_gpu
```
**What you'll learn**: Performance comparison, CPU vs GPU trade-offs, optimization basics

#### 3️⃣ **Low-Level Examples**
```bash
# Advanced performance analysis
cargo run --example performance_comparison_cpu
cargo run --example performance_comparison_gpu
```
**What you'll learn**: Advanced optimization, memory management, custom compute shaders

## 📊 Example Comparison

| Example | Complexity | Best For | Key Features |
|---------|------------|----------|--------------|
| **Quickstart** | ⭐ Beginner | Learning basics | 3 particles, simple physics, educational |
| **Three-Body** | ⭐⭐ Intermediate | Advanced physics | Celestial mechanics, orbital simulation |
| **High-Level CPU** | ⭐⭐ Intermediate | CPU optimization | 25 particles, performance metrics |
| **High-Level GPU** | ⭐⭐ Intermediate | GPU basics | 25 particles, compute shaders |
| **Low-Level CPU** | ⭐⭐⭐ Advanced | CPU mastery | Many particles, threading, profiling |
| **Low-Level GPU** | ⭐⭐⭐ Advanced | GPU mastery | Custom shaders, workgroup optimization |

## 🎯 Choose Your Path

### **I'm completely new to particle simulation**
→ Start with **Quickstart** to understand the fundamentals

### **I understand basic physics but want to learn the framework**
→ Try **Quickstart** then **Three-Body** for advanced physics

### **I want to see complex physics in action**
→ Jump to **Three-Body** for orbital mechanics and celestial dynamics

### **I want to compare CPU vs GPU performance**
→ Run both **High-Level CPU** and **High-Level GPU** examples

### **I need maximum performance for my project**
→ Study the **Low-Level Examples** for advanced optimization

### **I want to understand compute shaders**
→ Focus on **Low-Level GPU** examples

## 🔧 Prerequisites

### Required
- Rust 1.70+ installed
- Graphics card with wgpu support (most modern GPUs)
- ~100MB RAM for larger examples

### Recommended  
- Basic understanding of 3D coordinates (X, Y, Z)
- Familiarity with physics concepts (position, velocity, acceleration)
- Some Rust programming experience

## 🚀 Quick Commands

```bash
# Essential first run
cargo run --example quickstart

# Advanced physics demonstration
cargo run --example three_body

# See all available examples
cargo run --example --list

# Build all examples
cargo build --examples

# Run specific examples
cargo run --example simple_particles_cpu
cargo run --example simple_particles_gpu
cargo run --example performance_comparison_cpu
cargo run --example performance_comparison_gpu
```

## 📚 Documentation Structure

- **`quickstart/`** - Perfect starting point with step-by-step explanations
- **`simulation_usage/`** - Advanced examples organized by complexity
  - **`high_level/cpu/`** - CPU-focused intermediate examples  
  - **`high_level/gpu/`** - GPU-focused intermediate examples
  - **`low_level/cpu/`** - Advanced CPU optimization techniques
  - **`low_level/gpu/`** - Advanced GPU programming and shaders
- **`test/`** - 3D models and assets used by examples

## 💡 Learning Tips

1. **Start with quickstart** - Don't skip this even if you're experienced
2. **Read the code comments** - Every example is heavily documented
3. **Experiment freely** - Change values and see what happens
4. **Run examples side by side** - Compare CPU vs GPU performance
5. **Build incrementally** - Master one level before moving to the next

## 🤝 Getting Help

- **Read the README** in each example directory
- **Check code comments** for detailed explanations
- **Experiment with parameters** to understand behavior
- **Start simple** and gradually increase complexity

## 🌟 What's Special About Haggis

- **Educational Focus**: Examples designed for learning
- **Performance Comparison**: Direct CPU vs GPU examples
- **Production Ready**: From simple demos to high-performance simulations
- **Well Documented**: Every concept explained clearly
- **Modern Rust**: Leverages Rust's safety and performance

Ready to start? Run your first simulation:

```bash
cargo run --example quickstart
```

Have fun exploring the world of particle simulation! 🎉