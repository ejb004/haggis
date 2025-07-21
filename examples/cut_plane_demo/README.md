# 2D Cut Plane Visualization Demo

This example demonstrates the 2D cut plane visualization system in Haggis. It shows how to visualize 3D data by creating 2D cross-sections (slices) through the data volume.

## Overview

The demo creates a 3D test pattern consisting of concentric spheres with noise and gradient effects, then uses the cut plane visualization to explore this data interactively.

## Features Demonstrated

- **3D Data Generation**: Creates complex 3D patterns for visualization
- **Cut Plane Visualization**: Shows 2D slices through 3D data
- **Interactive Controls**: Real-time adjustment of visualization parameters
- **Multiple View Modes**: Different ways to visualize the data

## Running the Demo

```bash
cargo run --example cut_plane_demo
```

## User Interface

The demo includes several UI panels:

### Demo Info Panel (Left Side)
- Shows information about the current 3D pattern
- Displays data dimensions
- Provides usage instructions

### Cut Plane Visualization Panel (Right Side)
- **Plane Orientation**: Choose which plane to view:
  - XY: Looking down the Z-axis
  - XZ: Looking down the Y-axis  
  - YZ: Looking down the X-axis
- **Plane Position**: Slider to move through the data volume (0% to 100%)
- **Visualization Mode**: How to display the data:
  - **Heatmap**: Color-coded values (blue=low, red=high)
  - **Grid**: Grayscale representation
  - **Points**: High values shown as white dots
- **View Controls**: Zoom and pan within the 2D view
- **Action Buttons**: Reset view and center plane

### Visualization Manager Panel (Bottom Right)
- Enable/disable the entire visualization system
- Toggle individual visualization components

## Understanding the Visualization

### 3D Test Pattern
The demo generates a 3D pattern with:
- **Concentric Spheres**: Radial pattern emanating from the center
- **Noise**: Random variation for texture
- **Gradient**: Subtle directional trend

### Cut Plane Views
- **XY Plane**: Shows circular patterns when slicing through sphere centers
- **XZ Plane**: Shows how the pattern changes along the Y-axis
- **YZ Plane**: Shows how the pattern changes along the X-axis

### Visualization Modes
- **Heatmap**: Best for seeing value distributions and gradients
- **Grid**: Good for structural analysis
- **Points**: Useful for identifying high-value regions

## Code Structure

The example is organized into several key functions:

- `main()`: Sets up the application and visualization
- `generate_complex_test_pattern()`: Creates the main 3D pattern
- `generate_sine_wave_pattern()`: Alternative simpler pattern
- `generate_checkerboard_pattern()`: Alternative discrete pattern

## Extending the Demo

You can modify this demo to:

1. **Try Different Patterns**: Uncomment alternative pattern generators
2. **Add Real Data**: Replace test patterns with actual 3D datasets
3. **Customize UI**: Modify the demo info panel or add new controls
4. **Multiple Visualizations**: Add additional cut planes or other visualization types

## Technical Notes

- Data is stored in row-major order: `data[z * width * height + y * width + x]`
- Coordinate system follows Haggis convention (Z-up)
- Texture resolution is automatically determined from slice dimensions
- Color mapping uses a standard heat map (blue → cyan → green → yellow → red)

## Performance Considerations

- Larger data volumes will take more memory and processing time
- Texture updates happen when controls change
- GPU texture rendering is planned for future versions