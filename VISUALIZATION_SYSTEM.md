# Haggis Visualization System

This document describes the modular visualization system implemented in Haggis v0.1.0, with the 2D cut plane visualization as the first component.

## Overview

The visualization system provides a modular framework for adding various visualization components to Haggis applications. Each component manages its own rendering and UI controls, making the system extensible and reusable.

## Architecture

### Core Components

1. **`VisualizationComponent` trait** (`src/visualization/traits.rs`)
   - Defines the interface all visualization components must implement
   - Supports initialization, updates, UI rendering, and cleanup
   - Provides default implementations for optional methods

2. **`VisualizationManager`** (`src/visualization/manager.rs`)
   - Manages multiple visualization components
   - Handles GPU resource initialization
   - Provides centralized UI panel positioning
   - Offers master control panel for the visualization system

3. **`CutPlane2D`** (`src/visualization/cut_plane_2d.rs`)
   - First visualization component implementation
   - Shows 2D cross-sections of 3D data
   - Supports multiple plane orientations (XY, XZ, YZ)
   - Multiple visualization modes (Grid, Points, Heatmap)

4. **UI Controls** (`src/visualization/ui/`)
   - Specialized UI components for visualization controls
   - Cut plane controls with orientation, position, and mode selection
   - View controls for zoom and pan

### Integration Points

The visualization system integrates with Haggis at several levels:

1. **Application Level** (`src/app.rs`)
   - `HaggisApp` provides methods to add/remove visualizations
   - Visualization manager is part of the application state
   - GPU resources are initialized alongside simulations

2. **Event Loop Integration**
   - Visualizations are updated every frame
   - UI panels are rendered after simulation UI
   - Right-side panel positioning by default

3. **Module System** (`src/lib.rs`)
   - Visualization module is exported from the main library
   - Key types are re-exported for external use

## Usage

### Basic Setup

```rust
use haggis::{CutPlane2D, VisualizationComponent};

let mut app = haggis::default();

// Create and configure a cut plane visualization
let mut cut_plane = CutPlane2D::new();
cut_plane.set_data(&data, (width, height, depth));

// Add to the application
app.add_visualization("cut_plane", cut_plane);

app.run();
```

### Cut Plane Features

The 2D cut plane visualization provides:

- **Plane Orientations**: XY, XZ, YZ planes for different cross-section views
- **Position Control**: Slider to move through the data volume (0-100%)
- **Visualization Modes**:
  - **Heatmap**: Color-coded values (blue=low → red=high)
  - **Grid**: Grayscale representation
  - **Points**: High values shown as white dots
- **View Controls**: Zoom (0.1x-5x) and pan (-2 to +2 in both axes)
- **Action Buttons**: Reset view and center plane

### UI Panel Layout

The system automatically positions visualization panels:
- **Right side of screen** for visualization controls
- **Stacked vertically** when multiple components are active
- **Master control panel** at bottom-right for system-wide controls
- **Collapsible panels** to manage screen space

## Demo Application

The `cut_plane_demo` example demonstrates the complete system:

```bash
cargo run --example cut_plane_demo
```

Features:
- 3D concentric spheres test pattern (80×80×80)
- Interactive cut plane controls
- Demo info panel with instructions
- Real-time visualization updates

## Technical Details

### Data Format

3D data is stored in row-major order:
```rust
data[z * width * height + y * width + x]
```

Coordinate system follows Haggis convention (Z-up):
- X-axis: right
- Y-axis: forward  
- Z-axis: up

### Performance Considerations

- **Texture Updates**: Only when controls change (with change tracking)
- **Memory Usage**: Slice data is extracted on-demand
- **GPU Ready**: Infrastructure for future texture rendering
- **Configurable Resolution**: Texture size matches slice dimensions

### Extensibility

The system is designed for easy extension:

1. **New Visualization Types**: Implement `VisualizationComponent` trait
2. **Custom UI Controls**: Add to `src/visualization/ui/`
3. **Multiple Instances**: Same component type can be added multiple times
4. **Data Sources**: Components can receive data from simulations or external sources

## Future Enhancements

Planned improvements:
- **GPU Texture Rendering**: Direct GPU texture display in UI panels
- **Volume Rendering**: 3D volume visualization component
- **Particle Trails**: Visualization for particle system history
- **Custom Color Maps**: User-defined color schemes
- **Export Functionality**: Save visualizations as images
- **Animation Support**: Record and playback visualization sequences

## File Structure

```
src/visualization/
├── mod.rs                    # Module exports and documentation
├── traits.rs                 # VisualizationComponent trait
├── manager.rs                # VisualizationManager
├── cut_plane_2d.rs           # 2D cut plane implementation
└── ui/
    ├── mod.rs                # UI module exports
    └── cut_plane_controls.rs  # Cut plane UI controls

examples/cut_plane_demo/
├── main.rs                   # Demo application
└── README.md                 # Demo documentation
```

## Integration Status

✅ **Completed**:
- Core visualization framework
- 2D cut plane visualization
- UI controls and panels
- Demo application
- Documentation

🚧 **Future Work**:
- GPU texture rendering
- Additional visualization types
- Advanced interaction controls
- Performance optimizations

The modular design ensures that new visualization components can be added easily while maintaining clean separation of concerns and consistent user experience.