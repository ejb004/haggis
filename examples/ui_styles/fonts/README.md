# Fonts Directory

Place TTF font files here to use with the UiFont::Custom option.

## Example Usage

```rust
use haggis::{HaggisApp, UiFont};

let mut app = haggis::default();
app.set_ui_font(UiFont::Custom {
    data: include_bytes!("fonts/roboto.ttf"),
    size: 18.0,
});
```

## Recommended Fonts

- **Roboto**: Clean, modern sans-serif font
- **Source Code Pro**: Popular monospace font for code
- **Inter**: Highly legible UI font
- **JetBrains Mono**: Monospace font with programming ligatures

Note: Due to licensing, no fonts are included in this directory by default. Download fonts from their official sources.