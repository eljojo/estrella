# ⭐️ estrella 

A Rust library for [Star Micronics TSP650II](https://star-m.jp/eng/products/s_print/tsp650ii/index.html) thermal receipt printers over Bluetooth. Implements the [StarPRNT](https://starmicronics.com/support/download/starprnt-command-specifications/) protocol with a React-inspired component system, an optimizing compiler, and a web UI for photo printing.

![My Desk](https://github.com/user-attachments/assets/30b569ec-d311-492a-9333-d069e9234289)

## Photo Printing

The web UI supports printing photos with real-time dithered preview:

- **Formats:** JPEG, PNG, GIF, WEBP, HEIC (iPhone photos)
- **Adjustments:** Rotation, brightness, contrast
- **Dithering:** Choose algorithm for best results
- Auto-resize to 576px printer width

<img width="1125" height="1068" alt="Screenshot 2026-01-23 at 18 19 25" src="https://github.com/user-attachments/assets/d8e7779d-7940-47c6-a304-4fc6c7b2992e" />

## The Component System

Instead of manually constructing printer escape sequences, Estrella provides a declarative API inspired by React. You describe *what* you want, and the system figures out the bytes.

```rust
use estrella::components::*;

let receipt = Receipt::new()
    .child(Text::new("CHURRA MART").center().bold().size(2, 2))
    .child(Text::new("2026-01-20 12:00:00").center())
    .child(Spacer::mm(3.0))
    .child(Text::new(" TODAY ONLY: 50% OFF ").center().invert().bold())
    .child(Divider::dashed())
    .child(LineItem::new("Espresso", 4.50))
    .child(LineItem::new("Croissant", 3.25))
    .child(Divider::dashed())
    .child(Total::new(7.75).bold().double_width())
    .child(Spacer::mm(3.0))
    .child(QrCode::new("https://example.com/rewards").cell_size(6))
    .child(Text::new("Thank you!").center().bold())
    .cut();

let bytes = receipt.build();  // StarPRNT bytes, ready to send
```

![Demo Receipt](tests/golden/demo_receipt.png)

### Components

| Component | Description |
|-----------|-------------|
| `Text` | Styled text with `.bold()`, `.center()`, `.invert()`, `.size(w, h)`, etc. |
| `LineItem` | Left name + right price (e.g., "Coffee" ... "$4.50") |
| `Total` | Right-aligned total line |
| `Divider` | Horizontal line (`.dashed()`, `.solid()`, `.double()`) |
| `Spacer` | Vertical space in mm or lines |
| `Image` | Raster graphics with dithering |
| `QrCode`, `Pdf417`, `Barcode` | 1D and 2D barcodes |
| `NvLogo` | Logo from printer's flash memory |

## Dithering Algorithms

Thermal printers are binary (black or white), so grayscale images need dithering. Estrella implements four algorithms:

| Algorithm | Characteristics |
|-----------|-----------------|
| **Floyd-Steinberg** | Classic error diffusion. Smooth gradients, organic look. Default for photos. |
| **Atkinson** | Bill Atkinson's Mac algorithm. Higher contrast, loses 25% of error intentionally. |
| **Jarvis** | Spreads error over 12 neighbors. Smoothest gradients, slightly slower. |
| **Bayer** | Ordered 8x8 matrix. Fast, deterministic, halftone pattern. Best for patterns. |

| Floyd-Steinberg | Atkinson | Jarvis | Bayer |
|-----------------|----------|--------|-------|
| ![Floyd-Steinberg](tests/golden/dither_floyd_steinberg.png) | ![Atkinson](tests/golden/dither_atkinson.png) | ![Jarvis](tests/golden/dither_jarvis.png) | ![Bayer](tests/golden/dither_bayer.png) |

## Pattern Generation

![The web ui allows to preview patterns](https://github.com/user-attachments/assets/7a2d8847-0458-4a55-9044-65cd67a721d2)

Procedural patterns for artistic prints and printer calibration. Each pattern has randomizable parameters.

| ![Ripple](tests/golden/ripple.png) | ![Waves](tests/golden/waves.png) | ![Plasma](tests/golden/plasma.png) |
|:--:|:--:|:--:|
| Ripple | Waves | Plasma |

<details>
<summary>More patterns</summary>

**Op Art**

| ![Riley](tests/golden/riley.png) | ![Vasarely](tests/golden/vasarely.png) | ![Scintillate](tests/golden/scintillate.png) | ![Moire](tests/golden/moire.png) |
|:--:|:--:|:--:|:--:|
| Riley | Vasarely | Scintillate | Moire |

**Organic**

| ![Topography](tests/golden/topography.png) | ![Rings](tests/golden/rings.png) | ![Flowfield](tests/golden/flowfield.png) | ![Mycelium](tests/golden/mycelium.png) |
|:--:|:--:|:--:|:--:|
| Topography | Rings | Flowfield | Mycelium |

| ![Erosion](tests/golden/erosion.png) | ![Crystal](tests/golden/crystal.png) | ![Reaction Diffusion](tests/golden/reaction_diffusion.png) | ![Voronoi](tests/golden/voronoi.png) |
|:--:|:--:|:--:|:--:|
| Erosion | Crystal | Reaction Diffusion | Voronoi |

**Glitch**

| ![Glitch](tests/golden/glitch.png) | ![Corrupt Barcode](tests/golden/corrupt_barcode.png) | ![Databend](tests/golden/databend.png) | ![Scanline Tear](tests/golden/scanline_tear.png) |
|:--:|:--:|:--:|:--:|
| Glitch | Corrupt Barcode | Databend | Scanline Tear |

**Generative**

| ![Attractor](tests/golden/attractor.png) | ![Automata](tests/golden/automata.png) | ![Estrella](tests/golden/estrella.png) |
|:--:|:--:|:--:|
| Attractor | Automata | Estrella |

**Textures**

| ![Crosshatch](tests/golden/crosshatch.png) | ![Stipple](tests/golden/stipple.png) | ![Woodgrain](tests/golden/woodgrain.png) | ![Weave](tests/golden/weave.png) |
|:--:|:--:|:--:|:--:|
| Crosshatch | Stipple | Woodgrain | Weave |

**Calibration**

| ![Calibration](tests/golden/calibration.png) | ![Microfeed](tests/golden/microfeed.png) | ![Density](tests/golden/density.png) | ![Jitter](tests/golden/jitter.png) |
|:--:|:--:|:--:|:--:|
| Calibration | Microfeed | Density | Jitter |

| ![Overburn](tests/golden/overburn.png) |
|:--:|
| Overburn |

</details>

### Pattern Weaving

Blend multiple patterns with DJ-style crossfade transitions:

```bash
estrella weave ripple plasma waves --length 200mm --crossfade 30mm
```

![Weave Crossfade](tests/golden/weave_crossfade.png)

## JSON API

Send structured JSON documents to print anything the component library supports. Useful for automations (e.g. Home Assistant daily briefings).

```bash
curl -X POST http://localhost:8080/api/json/print \
  -H 'Content-Type: application/json' \
  -d '{
    "document": [
      {"type": "header", "content": "GOOD MORNING"},
      {"type": "text", "content": "Monday, January 27", "center": true, "font": "B"},
      {"type": "divider", "style": "double"},
      {"type": "text", "content": " WEATHER ", "bold": true, "invert": true},
      {"type": "columns", "left": "Now", "right": "6°C Cloudy"},
      {"type": "columns", "left": "High / Low", "right": "11°C / 3°C"},
      {"type": "divider"},
      {"type": "text", "content": " CALENDAR ", "bold": true, "invert": true},
      {"type": "columns", "left": "9:00", "right": "Standup"},
      {"type": "columns", "left": "11:30", "right": "Dentist"},
      {"type": "divider"},
      {"type": "qr_code", "data": "https://calendar.google.com"}
    ],
    "cut": true
  }'
```

The web UI includes a JSON API tab with a live preview editor and a sample daily briefing template.

**Endpoints:**
- `POST /api/json/preview` — returns a PNG preview
- `POST /api/json/print` — sends to printer

<details>
<summary>Full component reference</summary>

Each component in the `"document"` array has a `"type"` field and type-specific properties:

| Type | Required | Optional (defaults) |
|------|----------|---------------------|
| `text` | `content` | `bold`, `underline`, `upperline`, `invert`, `upside_down`, `reduced` (false); `smoothing` (null/auto); `align` ("left"), `center`, `right` (false); `font` ("A"); `size`, `scale` (null); `double_width`, `double_height` (false); `inline` (false) |
| `header` | `content` | `variant`: "normal" (2x2 centered bold) or "small" (1x1) |
| `line_item` | `name`, `price` | `width` (48) |
| `total` | `amount` | `label` ("TOTAL:"), `bold` (true), `double_width` (false), `align` ("right") |
| `divider` | — | `style`: "dashed" / "solid" / "double" / "equals"; `width` (48) |
| `spacer` | one of: `mm`, `lines`, `units` | — |
| `blank_line` | — | — |
| `columns` | `left`, `right` | `width` (48), `bold`, `underline`, `invert` (false) |
| `markdown` | `content` | `show_urls` (false) |
| `qr_code` | `data` | `cell_size` (4), `error_level` ("M"), `align` ("center") |
| `pdf417` | `data` | `module_width` (3), `ecc_level` (2), `align` ("center") |
| `barcode` | `format`, `data` | `height` (80); format: "code128" / "code39" / "ean13" / "upca" / "itf" |
| `pattern` | `name` | `height` (500), `params` ({}), `dither` ("bayer") |
| `nv_logo` | `key` | `center` (false), `scale` (1), `scale_x` (1), `scale_y` (1) |

**`size` / `scale` on text:** accepts a single number `2` (uniform 2x2) or an array `[2, 3]` for non-uniform height/width.

**`cut`** at the top level defaults to `true`. Set to `false` to suppress the paper cut.

</details>

## How It Works: The Compilation Pipeline

Components emit an intermediate representation (IR), which gets optimized before generating StarPRNT bytes:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              COMPILATION PIPELINE                           │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│   ┌─────────────┐      ┌─────────────┐      ┌─────────────┐      ┌───────┐  │
│   │  Components │      │     IR      │      │  Optimizer  │      │ Bytes │  │
│   │             │      │             │      │             │      │       │  │
│   │  Receipt    │      │  Op::Init   │      │  4 passes   │      │ ESC @ │  │
│   │  Text       │ ───► │  Op::Text   │ ───► │  that remove│ ───► │ ...   │  │
│   │  QrCode     │ emit │  Op::Bold   │      │  redundant  │ gen  │ 1D 69 │  │
│   │  ...        │      │  Op::Cut    │      │  operations │      │ ...   │  │
│   │             │      │  ...        │      │             │      │       │  │
│   └─────────────┘      └─────────────┘      └─────────────┘      └───────┘  │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Optimizer Passes

The optimizer runs four passes to eliminate redundant operations:

```
Pass 1: Remove Redundant Init     Pass 2: Collapse Style Toggles
┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄     ┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
Init  ←── keep                    SetBold(true)
...                               Text("Hello")
Init  ←── remove                  SetBold(false) ┐
...                               SetBold(true)  ┘── remove pair
Init  ←── remove                  Text("World")


Pass 3: Remove Redundant Styles   Pass 4: Merge Adjacent Text
┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄    ┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
SetBold(true)                     Text("Hello")  ┐
Text("A")                         Newline        │── merge into
SetBold(true) ←── remove          Text("World")  ┘   "Hello\nWorld"
Text("B")
```

**Result:** 11 ops → 6 ops, ~5-8% smaller output with identical visual results.

## Running at Home

### Prerequisites

- **Printer:** Star Micronics TSP650II (or compatible StarPRNT printer)
- **Connection:** Bluetooth, paired to create `/dev/rfcomm0`
- **Build:** Nix (recommended) or Rust 1.75+

### Quick Start

```bash
# With Nix
nix develop
cargo run -- serve
# Open http://localhost:8080

# Without Nix
cargo build --release
./target/release/estrella serve --device /dev/rfcomm0
```

### NixOS Module

For a proper deployment on NixOS:

```nix
{
  inputs.estrella.url = "github:eljojo/estrella";

  # Add to your flake outputs:
  nixpkgs.overlays = [ inputs.estrella.overlays.default ];

  # Enable the service:
  services.estrella = {
    enable = true;
    port = 8080;
    deviceMac = "00:11:22:33:44:55";  # Your printer's Bluetooth MAC
    # rfcommChannel = 0;  # Optional, defaults to 0
  };
}
```

The module creates two systemd services:
- `estrella-rfcomm.service` - Oneshot that sets up the Bluetooth RFCOMM device (runs as root)
- `estrella.service` - The HTTP daemon (runs unprivileged with DynamicUser)

### Bluetooth Setup

**Important:** The TSP650II ships in SSP (Secure Simple Pairing) mode which doesn't work with Linux. You need the **Star Settings** iOS/Android app to:
1. Disable "Auto Connect" (SSP mode)
2. Set a PIN code

Without this step, pairing will silently fail. This was the hardest part to debug.

**Pairing:**
```bash
bluetoothctl
> power on
> agent on
> default-agent
> scan on
# Find your printer's MAC address (starts with 00:12:F3 for Star)
> pair XX:XX:XX:XX:XX:XX
> trust XX:XX:XX:XX:XX:XX
> connect XX:XX:XX:XX:XX:XX
```

**Bind rfcomm device (automatic):**
```bash
sudo estrella setup-rfcomm XX:XX:XX:XX:XX:XX
# Connects, verifies with l2ping, and creates /dev/rfcomm0
```

**Bind rfcomm device (manual):**
```bash
sudo rfcomm bind 0 XX:XX:XX:XX:XX:XX 1
# Creates /dev/rfcomm0
```

**Permissions:** Add your user to the `dialout` group for `/dev/rfcomm0` access:
```bash
sudo usermod -aG dialout $USER
# Log out and back in
```

**Debug commands:**
```bash
bluetoothctl info XX:XX:XX:XX:XX:XX
l2ping -c 1 XX:XX:XX:XX:XX:XX
sdptool browse XX:XX:XX:XX:XX:XX
```

### CLI Reference

```bash
estrella print ripple              # Print a pattern
estrella print ripple --png out.png  # Preview to PNG
estrella print --list              # List patterns
estrella serve                     # Start web server
estrella weave ripple plasma --length 200mm  # Blend patterns
estrella logo store logo.png       # Store logo in NV memory
estrella setup-rfcomm XX:XX:XX:XX:XX:XX  # Set up Bluetooth RFCOMM (requires root)
```

<details>
<summary>Long Print Mode (Buffer Overflow Prevention)</summary>

### The Problem

Thermal printers have limited internal buffers (~100-200KB). Large images can overflow the buffer causing print failures.

### The Solution

Estrella automatically splits large images into multiple independent print jobs (~1000 rows each). Each job completes fully before the next begins, preventing buffer overflow.

### How It Works

```
Large Image (e.g., 2000 rows)
    ↓
split_for_long_print()
    ↓
Job 1: Init + Raster(rows 0-999)
    ↓ [1 second pause]
Job 2: Init + Raster(rows 1000-1999) + Feed + Cut
    ↓
Printer outputs seamless image
```

- Each job is a complete, independent StarPRNT program
- NO feed between jobs (image appears continuous)
- Feed/Cut only on the final job
- 1 second pause between jobs lets printer catch up

### Usage

Long print mode is **automatic** when using the web UI or CLI.

For programmatic use:
```rust
let programs = program.split_for_long_print();
transport.send_programs(&programs)?;
```

### Technical Details

| Parameter | Value |
|-----------|-------|
| Chunk size | 1000 rows (~125mm, ~72KB) |
| Pause between jobs | 1 second |
| Band mode alignment | 24-row boundaries |

</details>
