# Conchy Shot (`conchy-shot`)

> **Next-generation Wayland screenshot and annotation tool with unified multi-monitor canvas, built for [Conchy OS](https://github.com/conchy-os/conchy-os) in pure Rust.**

`conchy-shot` combines the beloved interactive workflow and rich toolset of **Flameshot** with a pure Rust, zero-lag Wayland-native architecture.

---

## ⚡ Why `conchy-shot` over Flameshot?

| Feature | Flameshot (C++ / Qt) | `conchy-shot` (Rust / Wayland) |
| :--- | :--- | :--- |
| **Language & Runtime** | C++17 + Heavy Qt6/Qt5 Stack | **100% Pure Rust** |
| **Dependencies Footprint** | **180 – 300+ MB** of Qt libraries | **<10 MB** standalone binary |
| **Multi-Monitor Canvas** | ❌ Broken / crashes / cursor offsets |  **Unified Cross-Monitor Canvas** (drag across screens seamlessly) |
| **Fullscreen Mode Impact** | ❌ Steals focus, breaks fullscreen games |  **Zero-Impact (`zwlr_layer_shell_v1` on Overlay layer)** |
| **Startup Latency** | 500 – 1500 ms (D-Bus / Portal roundtrips) | **<10 ms** (instant frame freeze) |
| **Configuration** | INI | **KDL (`shot.kdl`)** & TOML |

---

## ✨ Features

- **🌐 Unified Multi-Monitor Canvas (Default):**
  - Seamlessly select, crop, and annotate across multiple displays with different resolutions and scaling factors.
  - Easily switch to isolated per-monitor mode via `--per-output` or `multi-monitor "per-output"` in config.
- **🎨 Flameshot-Inspired Tool Ring / Radial Dock:**
  - Modern, dark-glass floating action ring positioned intelligently near the selection bounding box.
  - Instant tool switching, color cycling, and quick action buttons.
- **✏️ Comprehensive Annotation Tools:**
  - ↗️ **Arrow:** Directional arrow with smoothed angle and balanced head.
  - 🔲 **Rectangle:** Clean geometric box for framing UI elements.
  - ✎ **Pencil:** Smooth freehand drawing.
  - ▦ **Blur / Pixelate:** Mosaic block pixelation for obscuring passwords, tokens, and sensitive data.
  - ① **Step Markers:** Sequentially numbered badges (`1`, `2`, `3`...) for tutorials and bug reports.
  - **T** **Text:** Crisp Pango/Cairo typography.
  - ↶ **Undo Stack:** Full undo history (`Ctrl+Z`).
- **🛡️ Wayland Layer-Shell Overlay:**
  - Uses `zwlr_layer_shell_v1` at `Layer::Overlay` with exclusive input.
  - Never kicks games out of exclusive fullscreen, never interrupts video playback, and never causes screen flicker.
- **📋 Persistent Wayland Clipboard:**
  - Built-in clipboard server keeps screenshot data available in the Wayland clipboard (`wl-clipboard-rs`) even after the tool exits.

---

## ⌨️ Default Keybindings

| Key | Action |
| :--- | :--- |
| `Space` / `Return` | Confirm capture and save to disk / copy to clipboard |
| `Ctrl+C` | Copy to clipboard only (without saving to file) |
| `Ctrl+S` | Save to file and copy to clipboard |
| `Ctrl+Z` | Undo last annotation |
| `Escape` | Cancel / Dismiss overlay |
| `A` | Switch to **Arrow** tool |
| `R` | Switch to **Rectangle** tool |
| `P` | Switch to **Pencil** (freehand) tool |
| `B` | Switch to **Blur / Pixelate** tool |
| `S` | Switch to **Step Marker** tool |
| `T` | Switch to **Text** tool |
| `V` | Switch to **Select / Move** mode |
| `C` | Cycle drawing color palette |
| `Left` / `Right` / `Up` / `Down` | Nudge selection rectangle |
| `Shift` + Arrow Keys | Resize selection rectangle |
| `Tab` / `Shift+Tab` | Cycle focused monitor |

---

## ⚙️ Configuration (`shot.kdl`)

Place configuration at `~/.config/conchy/shot.kdl` (or `~/.config/conchy-shot/config.kdl`):

```kdl
// Conchy Shot Configuration

// Multi-monitor mode: "unified" (single canvas across all screens) or "per-output"
multi-monitor "unified"

// Save path pattern (supports strftime format)
save-path "~/Pictures/Screenshots/Screenshot_%Y-%m-%d_%H-%M-%S.png"

// Clipboard integration
copy-to-clipboard-on-save true

// UI Preferences
ui {
    ring-menu true
    theme "catppuccin-mocha" // "tokyo-night" | "catppuccin-mocha" | "nord" | "gruvbox"
    stroke-width 3.5
}
```

---

## 🚀 Usage

```bash
# Interactive selection (default when invoked with no arguments)
conchy-shot

# Interactive selection with explicit options
conchy-shot area --unified
conchy-shot area --per-output

# Fullscreen output capture
conchy-shot screen
conchy-shot screen --output DP-1

# Print resolved config path
conchy-shot config-path
```

---

## 📦 Building from Source

```bash
git clone https://github.com/conchy-os/conchy-shot.git
cd conchy-shot
cargo build --release
sudo cp target/release/conchy-shot /usr/local/bin/
```

---

## 📄 License

GPL-3.0-or-later. Built with pride for **Conchy OS**.
