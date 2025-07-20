# 🌌 Three-Body Problem Simulation

Welcome to an advanced celestial mechanics simulation demonstrating stable solutions to the famous **three-body problem**! This example showcases the Haggis framework's capability for complex multi-body physics simulations.

## 📖 What is the Three-Body Problem?

The three-body problem is one of the most famous problems in celestial mechanics and mathematical physics:

> **Given three celestial bodies with known masses, positions, and velocities, predict their future motion under mutual gravitational attraction.**

While the two-body problem has a closed-form analytical solution, the three-body problem generally does not. Most three-body systems exhibit **chaotic behavior**, making long-term prediction impossible. However, certain special configurations produce stable, periodic orbits.

## 🎯 What You'll See

This simulation demonstrates three different stable orbital configurations:

### 🌟 Figure-8 Orbit (Default)
- **Discovery**: Found by Carles Simó in 2000 using numerical methods
- **Configuration**: Three equal masses (m=1.0 each) following a figure-eight shaped path
- **Initial Conditions**: Uses exact researched values for guaranteed stability
- **Stability**: Perfectly periodic with a period of ~6.32 time units
- **Beauty**: One of the most elegant solutions to the three-body problem
- **Implementation**: Runge-Kutta 4th order integration with timestep dt=0.005

### 🔺 Triangular (Lagrange) Configuration  
- **Discovery**: Joseph-Louis Lagrange (1772) - theoretical solution
- **Configuration**: Three bodies at the vertices of an equilateral triangle
- **Stability**: Stable for certain mass ratios (demonstrated with equal masses)
- **Real Examples**: Jupiter's Trojan asteroids occupy similar L4/L5 points

### 🌌 Hierarchical System
- **Configuration**: Close binary pair + distant third body  
- **Stability**: Stable when third body is sufficiently distant
- **Real Examples**: Alpha Centauri system, many exoplanet systems
- **Dynamics**: Third body orbits the center of mass of the binary pair

## 🔬 Physics Implementation

### Gravitational Forces
The simulation implements **Newton's law of universal gravitation**:

```
F = G * (m1 * m2) / r²
```

Where:
- `F` = gravitational force between two bodies
- `G` = gravitational constant (scaled for simulation)
- `m1, m2` = masses of the two bodies  
- `r` = distance between the centers of mass

### Numerical Integration
We use **Runge-Kutta 4th order (RK4)** integration for high accuracy:

1. **Why RK4?** Higher-order methods provide better energy conservation
2. **Timestep Control**: Adaptive timesteps ensure numerical stability
3. **Conservation Laws**: The simulation preserves energy and momentum within numerical precision

### Initial Conditions
Each configuration uses carefully researched initial conditions:

- **Figure-8**: Exact values from Simó's 2000 discovery
  - Body 1: pos(0.9700436, -0.24308753, 0), vel(0.466203685, 0.43236573, 0)
  - Body 2: pos(-0.9700436, 0.24308753, 0), vel(0.466203685, 0.43236573, 0)  
  - Body 3: pos(0, 0, 0), vel(-0.932407370, -0.86473146, 0)
- **Triangular**: Calculated from Lagrange point theory
- **Hierarchical**: Derived from hierarchical triple star systems

## 🎮 Controls and Features

### Simulation Controls
- **Time Multiplier**: Speed up or slow down the simulation (0.1x to 3.0x)
- **Integration Step**: Adjust numerical precision (smaller = more accurate)
- **Configuration Buttons**: Switch between orbital configurations
- **Pause/Play**: Stop and resume the simulation
- **Reset**: Return to initial conditions

### Visual Features
- **Orbital Trails**: See the complete path each body has traveled
- **Real-time Statistics**: Monitor energy, momentum, and other physical quantities
- **Dynamic Scaling**: Body size reflects mass
- **Color Coding**: Each body has a distinct material and color

### Camera System
- **Mouse Rotation**: Drag to rotate around the system
- **Zoom**: Scroll wheel to zoom in/out
- **Auto-Follow**: Option to keep the system centered (camera follows center of mass)

## 📊 Educational Value

### Physics Concepts Demonstrated
1. **Conservation Laws**:
   - Total energy (kinetic + potential) remains constant
   - Linear momentum is conserved
   - Angular momentum is conserved

2. **Orbital Mechanics**:
   - Gravitational interactions
   - Stable vs. unstable orbits
   - Period and frequency analysis

3. **Numerical Methods**:
   - Integration techniques
   - Timestep sensitivity
   - Numerical stability

4. **Chaos Theory**:
   - Small changes can lead to dramatically different outcomes
   - Sensitive dependence on initial conditions
   - Islands of stability in chaotic systems

### Real-World Applications
- **Spacecraft Trajectory Planning**: NASA uses similar calculations for mission design
- **Exoplanet Discovery**: Understanding multi-star systems
- **Asteroid Tracking**: Predicting paths of near-Earth objects
- **Stellar System Evolution**: How star systems form and evolve

## 🚀 Running the Example

```bash
# Make sure you're in the haggis project directory
cd path/to/haggis

# Run the three-body simulation
cargo run --example three_body
```

## 🧪 Experiments to Try

### Stability Testing
1. **Modify Initial Conditions**: Change positions or velocities slightly
2. **Mass Variations**: Adjust body masses and observe stability
3. **Time Step Effects**: See how integration accuracy affects long-term behavior

### Configuration Exploration
1. **Figure-8 Variations**: Try scaling the orbit size
2. **Lagrange Points**: Experiment with mass ratios in triangular config
3. **Hierarchical Ratios**: Change the distance of the third body

### Physics Analysis
1. **Energy Conservation**: Monitor total energy over long periods
2. **Orbital Periods**: Time the complete orbits
3. **Center of Mass**: Observe how it remains stationary

## 🔧 Technical Implementation Details

### Code Structure
```
ThreeBodySimulation
├── bodies: Vec<CelestialBody>          // The three gravitating masses
├── physics: RK4 Integration            // Numerical solver
├── statistics: OrbitalStatistics       // Conservation law monitoring
└── visualization: Trail Rendering      // Orbital path display
```

### Performance Considerations
- **Timestep Selection**: Balance between accuracy and speed
- **Trail Management**: Limited trail length prevents memory growth
- **Update Frequency**: Statistics calculated periodically, not every frame
- **Numerical Stability**: Minimum distance check prevents singularities

### Mathematical Precision
- **Double Precision**: All calculations use f32 for real-time performance
- **Gravitational Constant**: Scaled for visible, stable motion
- **Initial Conditions**: Verified against published research

## 📚 Further Reading

### Scientific Papers
- Carles Simó: "New Families of Solutions in N-Body Problems" (2000)
- Alain Chenciner & Richard Montgomery: "A remarkable periodic solution of the three-body problem" (2000)

### Historical Context
- Henri Poincaré: Early work on three-body problem chaos (1890s)
- Lagrange Points: L1-L5 equilibrium points in restricted three-body problem
- KAM Theory: Kolmogorov-Arnold-Moser theorem on orbital stability

### Real-World Examples
- **Jupiter's Trojans**: Asteroids at L4/L5 Lagrange points
- **Earth-Moon-Sun System**: Restricted three-body problem
- **Exoplanet Systems**: Many discovered systems are hierarchical triples

## 🎯 Next Steps

After mastering this example:

1. **Extend to N-Body**: Try simulating more than three bodies
2. **Add Perturbations**: Include relativistic effects or solar wind
3. **Optimization**: Implement Barnes-Hut algorithm for large N
4. **Visualization**: Add force vectors, energy plots, or 3D trails
5. **Stability Analysis**: Implement Lyapunov exponent calculation

## 🌟 Why This Matters

The three-body problem represents the transition from **predictable** to **chaotic** dynamics in physics. This simulation demonstrates:

- **Mathematical Beauty**: Simple rules can create complex, elegant motion
- **Computational Physics**: How numerical methods unlock previously unsolvable problems  
- **Real Applications**: These same techniques guide spacecraft and predict asteroid paths
- **Educational Value**: Hands-on experience with fundamental physics concepts

Enjoy exploring the beautiful world of celestial mechanics! 🚀✨

---

*This simulation uses the Haggis framework to demonstrate advanced physics capabilities while remaining accessible and educational.*