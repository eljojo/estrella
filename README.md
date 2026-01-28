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

## Image Downloads

Documents can include images from URLs. When a document is submitted via the JSON API, images are automatically downloaded, cached, resized, and dithered for printing.

```json
{"type": "image", "url": "https://example.com/photo.jpg"}
```

- **Auto-resize:** Images are scaled to the printer's full width (576 dots) preserving aspect ratio
- **Max height:** Optional `height` field acts as a cap — if the resized image is taller, it shrinks to fit
- **Alignment:** Images narrower than paper width are centered by default (`"align": "center"`). Also accepts `"left"` or `"right"`
- **Dithering:** Defaults to Floyd-Steinberg. Set `"dither"` to `"bayer"`, `"atkinson"`, `"jarvis"`, or `"none"`
- **Caching:** Downloaded images are cached in memory and shared with photo sessions (30-min TTL), so previewing a document multiple times won't re-download

```json
{
  "type": "image",
  "url": "https://example.com/photo.jpg",
  "width": 400,
  "height": 300,
  "align": "center",
  "dither": "atkinson"
}
```

## The Document System

Instead of manually constructing printer escape sequences, Estrella provides a declarative `Document` model. The same types work for both Rust construction and JSON deserialization — one set of types, zero conversion layer.

```rust
use estrella::document::*;

let doc = Document {
    document: vec![
        Component::Banner(Banner::new("CHURRA MART")),
        Component::Text(Text { content: "2026-01-20 12:00:00".into(), center: true, ..Default::default() }),
        Component::Spacer(Spacer::mm(3.0)),
        Component::Banner(Banner { content: "TODAY ONLY: 50% OFF".into(), border: BorderStyle::Double, size: 2, ..Default::default() }),
        Component::Divider(Divider::default()),
        Component::LineItem(LineItem::new("Espresso", 4.50)),
        Component::LineItem(LineItem::new("Croissant", 3.25)),
        Component::Divider(Divider::default()),
        Component::Total(Total { amount: 7.75, bold: Some(true), double_width: true, ..Default::default() }),
        Component::Spacer(Spacer::mm(3.0)),
        Component::QrCode(QrCode { data: "https://example.com/rewards".into(), cell_size: Some(6), ..Default::default() }),
        Component::Text(Text { content: "Thank you!".into(), center: true, bold: true, ..Default::default() }),
    ],
    cut: true,
    ..Default::default()
};

let bytes = doc.build();                     // StarPRNT bytes, ready to send
let json = serde_json::to_string(&doc)?;     // Same type serializes to JSON
```

![Demo Receipt](tests/golden/demo_receipt.png)

### Components

| Component | Description |
|-----------|-------------|
| `Text` | Styled text (bold, center, invert, size 0–3, optional `font: "ibm"`) |
| `Header` | Pre-styled centered bold header |
| `Banner` | Framed text with box-drawing borders, auto-sizing (optional `font: "ibm"`) |
| `LineItem` | Left name + right price (e.g., "Coffee" ... "$4.50") |
| `Total` | Right-aligned total line |
| `Divider` | Horizontal line (dashed, solid, double, equals) |
| `Spacer` | Vertical space in mm, lines, or raw units |
| `Columns` | Two-column layout (left + right) |
| `Table` | Table with box-drawing borders, headers, per-column alignment |
| `Markdown` | Rich text from Markdown (headings, bold, lists) |
| `Image` | Image from URL (downloaded, cached, dithered, auto-centered) |
| `Pattern` | Generative art pattern with params |
| `Canvas` | Absolute-positioned raster compositing with blend modes |
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

The JSON API uses the same `Document` type as the Rust API — the component structs are all `Serialize + Deserialize`, so JSON documents map directly to Rust types with zero conversion. Useful for automations (e.g. Home Assistant daily briefings).

```bash
curl -X POST http://localhost:8080/api/json/print \
  -H 'Content-Type: application/json' \
  -d '{
    "document": [
      {"type": "banner", "content": "GOOD MORNING"},
      {"type": "text", "content": "Monday, January 27", "center": true, "size": 0},
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

Canvas components support absolute-positioned compositing with blend modes:

```json
{
  "type": "canvas",
  "height": 100,
  "elements": [
    {"type": "pattern", "name": "estrella", "height": 80, "position": {"x": -43, "y": 0}, "blend_mode": "add"},
    {"type": "text", "content": "Hello World", "center": true, "position": {"x": 7, "y": 16}, "blend_mode": "add"},
    {"type": "total", "amount": 0, "position": {"x": 0, "y": 34}, "blend_mode": "add"}
  ]
}
```

Elements without `position` stack top-to-bottom (flow mode). Dithering defaults to `"auto"` — Atkinson when continuous-tone content is detected, none otherwise.

**Endpoints:**
- `POST /api/json/preview` — returns a PNG preview
- `POST /api/json/print` — sends to printer

<details>
<summary>Full component reference</summary>

Each component in the `"document"` array has a `"type"` field and type-specific properties:

| Type | Required | Optional (defaults) |
|------|----------|---------------------|
| `text` | `content` | `bold`, `underline`, `upperline`, `invert`, `upside_down`, `reduced` (false); `smoothing` (null/auto); `align` ("left"), `center`, `right` (false); `size` (1, default Font A — 0=Font B, 2=double, 3=triple, or `[h,w]`); `scale` (null); `double_width`, `double_height` (false); `inline` (false); `font` (null — set `"ibm"` for IBM Plex Sans) |
| `header` | `content` | `variant`: "normal" (2x2 centered bold) or "small" (1x1) |
| `banner` | `content` | `size` (3, max expansion 0–3, auto-cascades width); `border`: "single"/"double"/"heavy"/"shade"/"shadow"; `bold` (true); `padding` (1); `font` (null — set `"ibm"` for IBM Plex Sans) |
| `line_item` | `name`, `price` | `width` (48) |
| `total` | `amount` | `label` ("TOTAL:"), `bold` (true), `double_width` (false), `align` ("right") |
| `divider` | — | `style`: "dashed" / "solid" / "double" / "equals"; `width` (48) |
| `spacer` | one of: `mm`, `lines`, `units` | — |
| `blank_line` | — | — |
| `columns` | `left`, `right` | `width` (48), `bold`, `underline`, `invert` (false) |
| `table` | `rows` | `headers` (null), `border`: "single"/"double"/"mixed"/"heavy"/"shade" (default: "single"); `align` ([] — per-column: "left"/"center"/"right"); `row_separator` (false); `width` (48) |
| `markdown` | `content` | `show_urls` (false) |
| `qr_code` | `data` | `cell_size` (4), `error_level` ("M"), `align` ("center") |
| `pdf417` | `data` | `module_width` (3), `ecc_level` (2), `align` ("center") |
| `barcode` | `format`, `data` | `height` (80); format: "code128" / "code39" / "ean13" / "upca" / "itf" |
| `image` | `url` | `dither` ("floyd-steinberg"), `width` (576), `height` (null), `align` ("center" — also "left", "right"; only affects images narrower than paper) |
| `pattern` | `name` | `height` (500), `params` ({}), `dither` ("bayer") |
| `canvas` | `elements` | `height` (auto), `width` (576), `dither` ("auto" — detects continuous-tone content); each element: `position` ({x, y}), `blend_mode` ("normal"), `opacity` (1.0) + any component fields |
| `nv_logo` | `key` | `center` (false), `scale` (1), `scale_x` (1), `scale_y` (1) |

**Text `size`** controls both font selection and character expansion using a 1-indexed model:

| Size | Font | Expansion | Chars/line | Description |
|------|------|-----------|-----------|-------------|
| `0` | B (9×24) | none | 64 | Small text |
| `1` | A (12×24) | none | 48 | Normal (default) |
| `2` | A | 2× | 24 | Double |
| `3` | A | 3× | 16 | Triple |

Pass a single number for uniform scaling (`"size": 2` = double height and width), or an `[h, w]` array for independent control (`"size": [3, 1]` = triple height, normal width, 48 chars/line).

The `banner` component uses the same sizing model but auto-selects the largest width that fits. Given `"size": 3`, it tries widths 3→2→1→Font B until the content fits inside the box-drawing frame.

**`cut`** at the top level defaults to `true`. Set to `false` to suppress the paper cut.

</details>

## How It Works: The Compilation Pipeline

Document components emit an intermediate representation (IR), which gets optimized before generating StarPRNT bytes:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              COMPILATION PIPELINE                           │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│   ┌─────────────┐      ┌─────────────┐      ┌─────────────┐      ┌───────┐  │
│   │  Document   │      │     IR      │      │  Optimizer  │      │ Bytes │  │
│   │             │      │             │      │             │      │       │  │
│   │  Text       │      │  Op::Init   │      │  4 passes   │      │ ESC @ │  │
│   │  LineItem   │ ───► │  Op::Text   │ ───► │  that remove│ ───► │ ...   │  │
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

## Fonts

The preview renderer embeds bitmap fonts for receipt rendering:

- **[Spleen](https://github.com/fcambus/spleen)** 12×24 — Font A (48 chars/line). Copyright (c) 2018-2024 Frederic Cambus. BSD 2-Clause license.
- **[UW ttyp0](https://people.mpi-inf.mpg.de/~uwe/misc/uw-ttyp0/)** 9×18 — Font B/C (64 chars/line, scaled vertically to 9×24 / 9×17). Copyright (c) 2012-2015 Uwe Waldmann. ttyp0 license (MIT-like).
- **[IBM Plex Sans](https://github.com/IBM/plex)** — Optional TTF font for Text and Banner components (`"font": "ibm"`). Anti-aliased rendering via `ab_glyph`, dithered to 1-bit. Copyright (c) IBM Corp. Apache 2.0 license.
