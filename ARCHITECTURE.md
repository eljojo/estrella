# Estrella Architecture Plan

> A Rust library and CLI for thermal receipt printing on Star Micronics printers via Bluetooth, with a focus on visual art, test-driven development, and extensibility.

## Vision

Estrella is a receipt printing platform with three layers:

1. **CLI** - Direct printer communication + PNG preview mode
2. **HTTP Daemon** - NixOS module/service listening for print requests
3. **Rust Library** - Core protocol implementation and declarative component system

### Target Hardware

Primary: **Star Micronics TSP650II** via Bluetooth (StarPRNT protocol)

Future: Configurable for other Star printers with different resolutions.

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                     Declarative Components                       │
│  Receipt, Text, Header, LineItem, Total, QrCode, Image, etc.    │
└─────────────────────────────────────────────────────────────────┘
                              │ emit()
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                 Intermediate Representation (IR)                 │
│  Op::Init, Op::SetBold, Op::Text, Op::Raster, Op::Cut, etc.    │
└─────────────────────────────────────────────────────────────────┘
                              │ optimize()
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Optimizer Passes                            │
│  1. Remove redundant init    3. Remove redundant styles         │
│  2. Collapse style toggles   4. Merge adjacent text             │
└─────────────────────────────────────────────────────────────────┘
                              │ to_bytes()
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                      StarPRNT Protocol                          │
│            ESC @, ESC E, ESC GS S, ESC d, etc.                  │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                   Bluetooth Transport                            │
│                      /dev/rfcomm0                                │
└─────────────────────────────────────────────────────────────────┘
```

---

## Crate Structure

```
estrella/
├── src/
│   ├── main.rs              # CLI entry point
│   ├── lib.rs               # Library root
│   │
│   ├── components/          # Declarative component system
│   │   ├── mod.rs           # Component trait, ComponentExt
│   │   ├── receipt.rs       # Receipt container
│   │   ├── text.rs          # Text, Header, LineItem, Total
│   │   ├── layout.rs        # Divider, Spacer, Columns, BlankLine
│   │   ├── graphics.rs      # Image, Pattern
│   │   ├── canvas.rs        # Canvas (absolute-positioned compositing)
│   │   └── barcode.rs       # QrCode, Pdf417, Barcode
│   │
│   ├── ir/                  # Intermediate representation
│   │   ├── mod.rs           # Module exports
│   │   ├── ops.rs           # Op enum, Program, StyleState
│   │   ├── optimize.rs      # Optimization passes
│   │   └── codegen.rs       # IR → StarPRNT bytes
│   │
│   ├── protocol/            # StarPRNT command builders (low-level)
│   │   ├── mod.rs
│   │   ├── commands.rs      # Base commands (init, cut, feed)
│   │   ├── text.rs          # Text styling commands
│   │   ├── graphics.rs      # Raster/band/NV commands
│   │   └── barcode/         # 1D/2D barcode commands
│   │       ├── mod.rs
│   │       ├── qr.rs        # QR code
│   │       ├── pdf417.rs    # PDF417
│   │       └── barcode1d.rs # Code39, Code128, EAN-13, etc.
│   │
│   ├── render/              # Image generation
│   │   ├── mod.rs
│   │   ├── dither.rs        # Bayer 8x8 ordered dithering
│   │   ├── patterns.rs      # Ripple, Waves, Sick, Calibration
│   │   └── ttf_font.rs      # TTF font rendering (IBM Plex Sans via ab_glyph)
│   │
│   ├── transport/           # Communication
│   │   └── bluetooth.rs     # RFCOMM transport with TTY config
│   │
│   ├── printer/             # Hardware abstraction
│   │   └── config.rs        # Printer specs (TSP650II: 576 dots, 203 DPI)
│   │
│   ├── receipt.rs           # Demo receipt templates (uses components)
│   └── error.rs             # Error types
│
├── tests/
│   ├── golden_tests.rs      # PNG and binary golden tests
│   └── golden/              # Golden test files (.png, .bin)
│
└── spec/                    # Reference Python implementations
    └── *.py
```

---

## Component System

The component system provides a React-like declarative API for building receipts:

```rust
use estrella::components::*;

let receipt = Receipt::new()
    .child(Text::new("CHURRA MART").center().bold().size(2, 2))
    .child(Text::new("2024-01-20 12:00:00").center())
    .child(Spacer::mm(3.0))
    .child(LineItem::new("Espresso", 4.50))
    .child(LineItem::new("Croissant", 3.25))
    .child(Divider::dashed())
    .child(Total::new(7.75))
    .child(QrCode::new("https://example.com"))
    .cut();

// Compile → Optimize → Generate bytes
let bytes = receipt.build();
```

### Component Trait

```rust
pub trait Component {
    fn emit(&self, ops: &mut Vec<Op>);
}

pub trait ComponentExt: Component {
    fn compile(&self) -> Program {
        let mut ops = vec![Op::Init];
        self.emit(&mut ops);
        Program { ops }
    }

    fn build(&self) -> Vec<u8> {
        self.compile().optimize().to_bytes()
    }
}
```

### Available Components

| Component | Description |
|-----------|-------------|
| `Receipt` | Root container (adds Init, optional Cut) |
| `Text` | Styled text (.bold(), .center(), .underline(), .size(), etc.) |
| `Header` | Pre-styled centered bold header |
| `LineItem` | Left name + right price formatting |
| `Total` | Right-aligned total with optional bold/double-width |
| `Divider` | Horizontal line (dashed, solid, double, equals) |
| `Spacer` | Vertical space (mm or lines) |
| `Image` | Raster graphics (.raster_mode() or .band_mode()) |
| `Pattern` | Named pattern (ripple, waves, sick, calibration) |
| `Canvas` | Absolute-positioned raster compositing (elements with position, blend_mode, opacity) |
| `QrCode` | QR code with cell_size and error level |
| `Pdf417` | PDF417 2D barcode |
| `Barcode` | 1D barcodes (code39, code128, ean13, upca, itf) |
| `Raw` | Escape hatch for direct Op or bytes |

---

## Intermediate Representation (IR)

The IR is a sequence of `Op` enums representing printer operations:

```rust
pub enum Op {
    // Printer Control
    Init,
    Cut { partial: bool },
    Feed { units: u8 },

    // Style Changes
    SetAlign(Alignment),
    SetFont(Font),
    SetBold(bool),
    SetUnderline(bool),
    SetInvert(bool),
    SetSize { height: u8, width: u8 },
    SetDoubleWidth(bool),
    SetDoubleHeight(bool),
    SetSmoothing(bool),
    SetUpperline(bool),
    SetUpsideDown(bool),
    SetReduced(bool),
    SetCodepage(u8),
    ResetStyle,

    // Content
    Text(String),
    Newline,
    Raw(Vec<u8>),

    // Graphics
    Raster { width: u16, height: u16, data: Vec<u8> },
    Band { width_bytes: u8, data: Vec<u8> },

    // Barcodes
    QrCode { data: String, cell_size: u8, error_level: QrErrorLevel },
    Pdf417 { data: String, module_width: u8, ecc_level: u8 },
    Barcode1D { kind: BarcodeKind, data: String, height: u8 },

    // NV Graphics (TODO)
    NvStore { key: String, width: u16, height: u16, data: Vec<u8> },
    NvPrint { key: String, scale_x: u8, scale_y: u8 },
    NvDelete { key: String },
}
```

---

## Optimizer

The optimizer (`src/ir/optimize.rs`) runs four passes to reduce output size:

### Pass 1: Remove Redundant Init
Keep only the first `Op::Init`, remove duplicates.

### Pass 2: Collapse Style Toggles
Remove off+on pairs that cancel out:
```
SetBold(false), SetBold(true) → [removed]
SetSize{0,0}, SetSize{1,1} → SetSize{1,1}
```

### Pass 3: Remove Redundant Styles
Skip style changes that match current state:
```
State: bold=false
SetBold(false) → [skipped - already false]
SetBold(true) → [emitted - changes state]
```

### Pass 4: Merge Adjacent Text
Combine consecutive `Op::Text` operations:
```
Text("Hello"), Text(" "), Text("World") → Text("Hello World")
```

**Results:** 5-8% smaller output than hand-written byte sequences.

---

## Analysis of Python Scripts

The `spec/` folder contains Python scripts demonstrating printer capabilities:

| Script | Purpose | Key Feature |
|--------|---------|-------------|
| `star-prnt.py` | Ripple pattern | ESC GS S raster, Bayer dithering |
| `print_tests.py` | Test suite | Alignment, density, overburn, jitter |
| `nv_logo_store.py` | Store NV graphic | ESC GS ( L function 67 |
| `nv_logo_print.py` | Print NV graphic | ESC GS ( L function 69 with scaling |
| `ripples.py` | Ripple via band mode | ESC k (24-row bands) |
| `page.py` | Page mode layout | ESC GS P commands, absolute positioning |
| `receipt2.py` | Full receipt | Text styles, barcodes, QR, PDF417 |
| `asb-test.py` | Auto-Status-Back | Status queries, async reads |
| `receipt.py` | Simple receipt | Basic text styling |
| `sick.py` | Calibration pattern | Borders, diagonals, bars |
| `waves.py` | Wavy interference | Multi-oscillator pattern |
| `demo.py` | Wavy (simplified) | Same as waves, file-based I/O |

---

## CLI Design

### Commands

```
estrella print [OPTIONS] <PATTERN>
estrella print --list
```

### Print Subcommand

```
estrella print [OPTIONS] <PATTERN>

Arguments:
  <PATTERN>  Pattern or receipt to print (ripple, waves, sick, receipt, etc.)

Options:
  --list               List available patterns and receipts
  --png <FILE>         Output to PNG instead of printer (preview mode)
  --device <PATH>      Printer device path [default: /dev/rfcomm0]
  --width <DOTS>       Override print width [default: 576]
  --height <ROWS>      Pattern height in rows [default: pattern-specific]
  --no-title           Skip printing title header for patterns
  --band               Use band mode instead of raster mode
```

---

## Testing

### Golden Tests

Golden tests verify binary output consistency:

```rust
#[test]
fn test_ripple_golden() {
    let pattern = patterns::Ripple::default();
    let (width, height) = pattern.default_dimensions();
    let raster = pattern.render(width, height);
    let png = raster_to_png(width, height, &raster);

    let golden = include_bytes!("golden/ripple_576x500.png");
    assert_eq!(png, golden);
}

#[test]
fn test_binary_golden_demo_receipt() {
    let cmd = receipt::demo_receipt();
    let golden = fs::read("tests/golden/demo_receipt.bin").unwrap();
    assert_eq!(cmd, golden);
}
```

Run `make golden` to regenerate golden files after intentional changes.

---

## Implementation Phases

### Phase 1: Core Protocol - COMPLETE

- [x] Set up Cargo.toml with dependencies (clap, image, thiserror)
- [x] Implement `protocol/commands.rs` - basic StarPRNT commands
- [x] Implement `protocol/graphics.rs` - band and raster modes
- [x] Implement `transport/bluetooth.rs` - RFCOMM communication
- [x] Implement `render/dither.rs` - Bayer dithering
- [x] Implement `render/patterns.rs` - Ripple pattern
- [x] Basic CLI: `estrella print ripple`
- [x] Tests for all protocol commands

### Phase 2: Full Kitchensink - COMPLETE

- [x] Port all Python patterns (waves, sick, calibration)
- [x] Implement `protocol/text.rs` - text styling
- [x] Implement `protocol/barcode.rs` - 1D and 2D barcodes
- [x] `--list` command to show available patterns
- [x] `--png` preview mode
- [x] Golden tests for each pattern

### Phase 3: Component System and IR - COMPLETE

- [x] Design IR opcodes (`Op` enum)
- [x] Implement component trait and ComponentExt
- [x] Build core components (Receipt, Text, LineItem, Total, etc.)
- [x] Implement optimizer passes
- [x] Implement codegen (IR → bytes)
- [x] Migrate receipts to use components
- [x] Migrate CLI pattern printing to use components
- [x] Binary golden tests for command output

### Phase 4: HTTP Daemon

- [ ] HTTP server (axum or actix)
- [ ] REST API for print jobs
- [ ] Template selection via API
- [ ] NixOS module

### Phase 5: Advanced Features

- [ ] NV graphics storage/recall (Op::NvStore, NvPrint, NvDelete)
- [ ] Page mode support
- [ ] ASB status monitoring
- [ ] Multiple printer support
- [ ] Web preview interface
- [ ] Image loading and processing

---

## Printer Protocol Quick Reference

### Escape Sequences

| Command | Bytes | Description |
|---------|-------|-------------|
| Init | `1B 40` | Initialize printer |
| Cut Full | `1B 64 02` | Feed and full cut |
| Cut Partial | `1B 64 03` | Feed and partial cut |
| Feed n/4mm | `1B 4A n` | Micro feed |
| Align Left | `1B 1D 61 00` | Text alignment |
| Align Center | `1B 1D 61 01` | Text alignment |
| Align Right | `1B 1D 61 02` | Text alignment |
| Bold On | `1B 45` | Emphasis on |
| Bold Off | `1B 46` | Emphasis off |
| Underline | `1B 2D n` | n=0 off, n=1 on |
| Invert On | `1B 34` | White on black |
| Invert Off | `1B 35` | Black on white |
| Font | `1B 1E 46 n` | n=0 A, n=1 B, n=2 C |
| Band | `1B 6B n1 n2 data` | 24-row band (n1=width bytes) |
| Raster | `1B 1D 53 m xL xH yL yH n data` | Monochrome raster |

### Print Dimensions (TSP650II)

- Paper width: 80mm (with 72mm printable)
- Print width: 576 dots (72 bytes)
- Resolution: 203 DPI (~8 dots/mm)
- Band height: 24 rows

---

## Visual Art Patterns

### Ripple Formula
```
r = sqrt((x - cx)^2 + (y - cy)^2)
ripple = 0.5 + 0.5 * cos(r / scale - y / drift)
wobble = 0.5 + 0.5 * sin(x / 37.0 + 0.7 * cos(y / 53.0))
v = ripple * (1 - mix) + wobble * mix
```

### Waves Formula
```
horiz = sin(x / 19.0 + 0.7 * sin(y / 37.0))
vert = cos(y / 23.0 + 0.9 * cos(x / 41.0))
radial = cos(r / 24.0 - y / 29.0)
v = 0.45 * horiz + 0.35 * vert + 0.20 * radial
```

### Gamma Correction
```
v_corrected = v ^ gamma  // gamma typically 1.25-1.35
```

---

## Future Ideas

- **Perlin noise patterns** - organic, cloud-like textures
- **Floyd-Steinberg dithering** - alternative to Bayer
- **Photo printing** - load images, apply dithering
- **Generative art** - cellular automata, L-systems
- **Receipt templates** - declarative receipt definitions in TOML/YAML
- **Live preview** - web UI showing real-time changes
- **Multi-printer** - route jobs to different printers

---

*This document serves as the architectural guide for estrella development. AI assistants continuing this work should reference this plan for context and direction.*
