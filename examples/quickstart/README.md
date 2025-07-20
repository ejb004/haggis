# 🚀 Haggis Framework Quickstart

Welcome to the Haggis particle simulation framework! This quickstart example is your first step into creating amazing particle simulations.

## What You'll Learn

This example demonstrates the core concepts you need to understand:

- **Basic Application Setup**: How to initialize the Haggis framework
- **Scene Management**: Adding 3D objects and materials
- **Particle Simulation**: Creating and updating particles with physics
- **User Interface**: Building interactive controls
- **Rendering Integration**: Connecting simulation to visuals

## 🎯 What You'll See

When you run this example, you'll see:
- 3 colorful bouncing cubes (red, green, blue)
- A gray ground plane for visual reference
- Realistic gravity and collision physics
- Interactive controls to adjust physics parameters
- Real-time particle count and simulation info
- Smooth 3D graphics with camera controls

## 🏃‍♂️ Running the Example

```bash
# Make sure you're in the haggis project directory
cd path/to/haggis

# Run the quickstart example
cargo run --example quickstart
```

## 🎮 Controls

### Camera Controls
- **Mouse Movement**: Look around the 3D scene
- **Mouse Scroll**: Zoom in and out
- **Shift + Mouse**: Pan the view

### Physics Controls (UI Panel)
- **Gravity Slider**: Change how strong gravity is
- **Bounce Damping**: Control how bouncy collisions are
- **Ground Level**: Adjust where the ground is
- **Pause/Play Button**: Stop and start the simulation
- **Reset Button**: Return to initial state

## 📖 Understanding the Code

The quickstart example is heavily commented to help you understand each part:

### Key Components

1. **SimpleParticle Struct**: Represents a single particle
   ```rust
   struct SimpleParticle {
       position: Vector3<f32>,    // Where it is
       velocity: Vector3<f32>,    // How it's moving
       active: bool,              // Whether it's simulated
   }
   ```

2. **QuickstartSimulation**: Main simulation logic
   - Handles physics updates
   - Manages particle lifecycle
   - Syncs with visual objects

3. **Main Function**: Application setup
   - Creates the Haggis app
   - Adds materials and objects
   - Configures the simulation

### Physics Simulation Loop

Each frame, the simulation:
1. **Applies gravity** to particle velocities
2. **Updates positions** based on velocities
3. **Handles collisions** with ground and boundaries
4. **Syncs visual objects** to particle positions

## 🔧 Customization Ideas

Try modifying the code to experiment:

### Easy Changes
- Change the number of particles (add more objects in `main()`)
- Modify colors by adjusting material RGB values
- Change initial positions and velocities
- Adjust physics constants (gravity, damping, boundaries)

### Medium Changes
- Add different shapes (try loading different .obj files)
- Implement particle-to-particle collisions
- Add wind or other forces
- Create particle trails or effects

### Advanced Changes
- Add particle spawning and despawning
- Implement different physics models
- Create particle systems (fireworks, explosions, etc.)
- Add complex UI controls

## 📚 Next Steps

Once you're comfortable with this example:

1. **Explore High-Level Examples**: Check out `examples/simulation_usage/high_level/`
   - See CPU vs GPU performance comparisons
   - Learn advanced particle features

2. **Try Low-Level Examples**: Look at `examples/simulation_usage/low_level/`
   - Advanced optimization techniques
   - Custom compute shaders
   - Manual memory management

3. **Read the Documentation**: Study the framework's capabilities
   - Material system
   - Rendering pipeline
   - Advanced simulation features

## 🧠 Key Concepts Explained

### Coordinate System
- **X**: Left (-) to Right (+)
- **Y**: Down (-) to Up (+) 
- **Z**: Away (-) to Towards (+)

### Physics Integration
- **Position = Position + Velocity × Time**
- **Velocity = Velocity + Acceleration × Time**
- **Gravity affects acceleration, which affects velocity, which affects position**

### Frame Rate Independence
- All physics use `delta_time` to ensure consistent behavior regardless of frame rate
- 60 FPS vs 30 FPS should look the same, just smoother

## ❓ Troubleshooting

### Common Issues

**"No particles visible"**
- Check that objects were added to the scene in `main()`
- Verify particles are marked as `active: true`
- Make sure `sync_particles_to_scene()` is being called

**"Particles flying away"**
- Reduce initial velocities
- Check boundary constraints
- Verify gravity is negative (downward)

**"Performance issues"**
- This example uses only 3 particles for simplicity
- For many particles, consider the high-level or low-level examples

**"Compilation errors"**
- Ensure you're in the correct directory
- Run `cargo build` first to check for dependency issues
- Check that the `examples/test/cube.obj` file exists

## 💡 Tips for Learning

1. **Read the comments**: Every important concept is explained in the code
2. **Experiment freely**: Change values and see what happens
3. **Start simple**: Master this example before moving to complex ones
4. **Use the debugger**: Set breakpoints to see how values change
5. **Ask questions**: The code is designed to be educational

## 🌟 What Makes This Special

This quickstart demonstrates the power of the Haggis framework:
- **Minimal boilerplate**: Just focus on your simulation logic
- **Automatic rendering**: 3D graphics just work
- **Built-in UI**: No need to learn separate UI frameworks
- **Performance**: Efficient enough for real-time interaction
- **Extensible**: Easy to build complex simulations from this foundation

Happy simulating! 🎉