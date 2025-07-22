# UI Styles Example

This example demonstrates the different UI styling options available in Haggis. You can choose between predefined themes or create custom color schemes.

## Features Demonstrated

- **Light Theme**: Bright backgrounds with dark text (default)
- **Dark Theme**: Dark backgrounds with light text
- **Matrix Theme**: Green-on-black Matrix-inspired colors
- **Custom Theme**: User-defined color palette
- **Font Options**: Default, Monospace, and Custom TTF fonts
- **Sample Graph**: Demonstrates how styles affect plots and controls

## Available UI Styles

### Predefined Styles

```rust
use haggis::{HaggisApp, UiStyle};

let mut app = haggis::default();

// Light theme (default)
app.set_ui_style(UiStyle::Light);

// Dark theme
app.set_ui_style(UiStyle::Dark);

// Matrix theme (green on black)
app.set_ui_style(UiStyle::Matrix);

// ImGui default theme
app.set_ui_style(UiStyle::Default);
```

### Custom Styles

```rust
app.set_ui_style(UiStyle::Custom {
    background: [0.2, 0.3, 0.4, 1.0],      // Window background
    text: [1.0, 1.0, 1.0, 1.0],            // Text color
    button: [0.4, 0.6, 0.8, 1.0],          // Button color
    button_hovered: [0.5, 0.7, 0.9, 1.0],  // Hovered button
    button_active: [0.3, 0.5, 0.7, 1.0],   // Active button
});
```

### Font Options

```rust
use haggis::{HaggisApp, UiFont};

let mut app = haggis::default();

// Default font
app.set_ui_font(UiFont::Default);

// Monospace font (fallback)
app.set_ui_font(UiFont::Monospace);

// Custom TTF font
app.set_ui_font(UiFont::Custom {
    data: include_bytes!("fonts/roboto.ttf"),
    size: 16.0,
});
```

## Running the Example

```bash
cargo run --example ui_styles
```

## Usage

1. Run the example to see the light theme (default)
2. Modify the `main.rs` file to try different options:
   - Change `UiStyle::Matrix` to `UiStyle::Light` or `UiStyle::Dark`
   - Try the custom theme example
   - Switch between font options (Default, Monospace, Custom)
   - Add your own TTF font files to the `fonts/` directory
3. Observe how all UI panels (transform controls, simulation panels, etc.) use the same theme and font

## Key Points

- UI styles and fonts are set globally and apply to all interface elements
- Both style and font are applied when the UI manager is initialized
- You can change both style and font before calling `app.run()`
- Custom themes allow precise control over individual color elements
- Custom fonts support any TTF file with configurable size
- Font changes affect all text rendering including graphs and controls