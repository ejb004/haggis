# UI Styles Example

This example demonstrates the different UI styling options available in Haggis. You can choose between predefined themes or create custom color schemes.

## Features Demonstrated

- **Light Theme**: Bright backgrounds with dark text (default)
- **Dark Theme**: Dark backgrounds with light text
- **Matrix Theme**: Green-on-black Matrix-inspired colors
- **Custom Theme**: User-defined color palette
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

## Running the Example

```bash
cargo run --example ui_styles
```

## Usage

1. Run the example to see the light theme (default)
2. Modify the `main.rs` file to try different styles:
   - Change `UiStyle::Light` to `UiStyle::Dark`
   - Try the custom theme example
   - Create your own custom colors
3. Observe how all UI panels (transform controls, simulation panels, etc.) use the same theme

## Key Points

- UI styles are set globally and apply to all interface elements
- Styles are applied when the UI manager is initialized
- You can change styles before calling `app.run()`
- Custom themes allow precise control over individual color elements