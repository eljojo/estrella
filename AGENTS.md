# Estrella

Rust library and CLI for Star Micronics thermal receipt printers (TSP650II) via Bluetooth. Implements the StarPRNT protocol for text formatting, graphics rendering, and barcode printing.


Remember:
- understand the codebase deeply before changing it. Whether it's a new feature, a bugfix, or a refactor, first understand how the existing pieces fit together. The right fix often comes from seeing how the system already works, not from adding new code on top. A bug might reveal that an abstraction needs reshaping, not just a patch.
- first try to reuse the code that's already written, then try to extend what's already written, only after that write from scratch
- unused code cost us 10x more than used code, so we need to avoid it as much as possible
- new code/concepts has an exponential cognitive load, so we need to try reusing existing code/concepts as much as possible.
- build with a vision instead of short sighted hacks that only solve the immediate problem
- when something doesn't fit the existing abstractions, refactor the abstractions rather than working around them. If a change requires `if special_case` branches scattered across the codebase, the abstraction is wrong — reshape it so the new thing is a natural citizen. This applies equally to features, bugfixes, and refactors. Prefer deep integration that makes the whole codebase fit together better over bolt-ons that duplicate similar logic in a new place.
- NEVER handle git write operations (commit, push, rebase, merge, etc.) - git add is allowed only to debug flake issues

## Quick Reference

```bash
/usr/bin/make build          # Release build
/usr/bin/make test           # Run tests
/usr/bin/make golden         # Regenerate golden test files
nix develop --command cargo run -- print --list  # List available patterns
nix develop --command cargo run -- print ripple --png out.png  # Preview to PNG
```

## Development Environment

**ALL commands must run through `nix develop`.** The nix flake provides cargo, node, npm, npx, and native dependencies (libheif, etc.). Nothing works without it.
```bash
nix develop --command cargo check
nix develop --command cargo test
nix develop --command cargo run -- serve
nix develop --command bash -c 'cd frontend && npx playwright test'
```

For multi-command or complex invocations, wrap in bash:
```bash
nix develop --command bash -c 'cd frontend && npm install && npm run build'
```

**Use `nix-shell -p` for ad-hoc tools not in the flake:**
```bash
nix-shell -p python3Packages.pillow --run "python3 script.py"
nix-shell -p imagemagick --run "convert ..."
```

Do NOT run bare `cargo`, `npm`, `npx`, or other toolchain commands outside of `nix develop` — they will either fail or use the wrong versions.

## Architecture

```
Document (declarative) → IR (opcodes) → Optimizer → Codegen → StarPRNT Bytes
```

### Pipeline

1. **Document** (`src/document/`): Unified declarative API (Rust structs + JSON serde)
2. **IR** (`src/ir/ops.rs`): Intermediate representation as `Op` enum
3. **Optimizer** (`src/ir/optimize.rs`): Removes redundant operations
4. **Codegen** (`src/ir/codegen.rs`): Converts IR to StarPRNT bytes

## Project Structure

```
src/
├── document/       # Unified document model (Rust API + JSON API)
│   ├── mod.rs      # Document struct, Component enum, compile/build
│   ├── types.rs    # All component structs (serde Serialize + Deserialize)
│   ├── text.rs     # emit() for Text, Header, LineItem, Total
│   ├── layout.rs   # emit() for Divider, Spacer, BlankLine, Columns, Banner, Table
│   ├── graphics.rs # emit() for Image, Pattern, NvLogo
│   ├── barcode.rs  # emit() for QrCode, Pdf417, Barcode
│   └── markdown.rs # emit() for Markdown
├── ir/             # Intermediate representation
│   ├── mod.rs      # Module exports
│   ├── ops.rs      # Op enum, Program, StyleState
│   ├── optimize.rs # Optimization passes
│   └── codegen.rs  # IR → StarPRNT bytes
├── protocol/       # StarPRNT command builders (low-level)
│   ├── commands.rs # Base commands (init, cut, feed)
│   ├── text.rs     # Text styling (alignment, fonts, emphasis)
│   ├── graphics.rs # Raster and band mode graphics
│   └── barcode/    # 1D/2D barcodes (QR, PDF417, Code39)
├── render/         # Image generation
│   ├── patterns.rs # Visual patterns (ripple, waves, sick)
│   └── dither.rs   # Bayer 8x8 ordered dithering
├── transport/      # Communication
│   └── bluetooth.rs # RFCOMM transport with TTY config
├── printer/        # Hardware abstraction
│   └── config.rs   # Printer specs (TSP650II: 576 dots, 203 DPI)
├── receipt.rs      # Demo receipt templates (uses Document)
└── main.rs         # CLI entry point
```

## Document System

Build documents declaratively. The same types work for Rust construction and JSON deserialization:

```rust
use estrella::document::*;

let doc = Document {
    document: vec![
        Component::Text(Text { content: "HEADER".into(), center: true, bold: true, size: [3, 3], ..Default::default() }),
        Component::LineItem(LineItem::new("Coffee", 4.50)),
        Component::Total(Total::new(4.50)),
        Component::QrCode(QrCode::new("https://example.com")),
    ],
    cut: true,
    ..Default::default()
};

let bytes = doc.build();                     // StarPRNT bytes
let json = serde_json::to_string(&doc)?;     // JSON (for APIs, storage)
```

### Available Components

| Component | Description |
|-----------|-------------|
| `Text` | Styled text (bold, center, underline, size 0–3, optional `font: "ibm"` for IBM Plex Sans) |
| `Header` | Pre-styled centered bold header |
| `Banner` | Framed text with box-drawing borders (single, double, heavy, shade, shadow), auto-sizing, optional `font: "ibm"` |
| `LineItem` | Left name + right price |
| `Total` | Right-aligned total with optional bold/double-width |
| `Divider` | Horizontal line (dashed, solid, double, equals) |
| `Spacer` | Vertical space (mm, lines, or raw units) |
| `Columns` | Two-column layout (left + right) |
| `Table` | Table with box-drawing borders (single, double, mixed, heavy, shade), headers, per-column alignment |
| `Markdown` | Markdown content (headings, bold, italic, lists) |
| `Image` | Image from URL (downloaded + cached, dithered for printing, align: left/center/right, default center) |
| `Pattern` | Named pattern (ripple, waves, etc.) with params |
| `QrCode` | QR code with cell_size and error level |
| `Pdf417` | PDF417 2D barcode |
| `Barcode` | 1D barcodes (code39, code128, ean13, upca, itf) |
| `Canvas` | Absolute-positioned raster compositing (elements with position, blend_mode, opacity; auto-dither) |
| `NvLogo` | Logo from printer's flash memory |

## IR Opcodes

The `Op` enum represents all printer operations:

```rust
Op::Init                           // ESC @
Op::Cut { partial: bool }          // ESC d n
Op::Feed { units: u8 }             // ESC J n (units = 1/4mm)
Op::SetAlign(Alignment)            // ESC GS a n
Op::SetBold(bool)                  // ESC E / ESC F
Op::SetUnderline(bool)             // ESC - n
Op::Text(String)                   // Raw text bytes
Op::Newline                        // 0x0A
Op::Raster { width, height, data } // ESC GS S
Op::Band { width_bytes, data }     // ESC k
Op::QrCode { data, cell_size, error_level }
Op::Barcode1D { kind, data, height }
// ... see src/ir/ops.rs for full list
```

## Optimizer Passes

The optimizer (`src/ir/optimize.rs`) runs four passes:

1. **remove_redundant_init**: Keep only the first `Op::Init`
2. **collapse_style_toggles**: Remove off+on pairs (e.g., `SetBold(false), SetBold(true)` → removed)
3. **remove_redundant_styles**: Skip style changes that match current state
4. **merge_adjacent_text**: Combine consecutive `Op::Text` into one

This produces ~5-8% smaller output than hand-written byte sequences.

## StarPRNT Protocol Specification

**The official spec lives at:** `spec/docs/starprnt_cm_en-*.html`

This is the StarPRNT Command Specifications Rev. 4.10 from Star Micronics. When writing or modifying protocol code, always:

1. **Reference the spec section** - Include page/section numbers in comments
2. **Use exact command names** - Match the spec's terminology (e.g., "ESC GS a n" not "set alignment")
3. **Document parameter ranges** - The spec defines valid values; document them
4. **Note model variations** - Some commands vary by printer model

There's also mostly-working demo implementations in python under `spec/*.py`, useful for comparison.

### Navigating the Spec

The spec is a PDF-to-HTML conversion. Each page is a separate file with absolute-positioned text elements.

- **Outline/TOC:** `spec/docs/starprnt_cm_en-outline.html` — lists all sections with links to page files. Start here to find which page a topic is on.
- **Single-file version:** `spec/docs/starprnt_cm_ens.html` — all pages concatenated. Best for grepping across the whole spec (e.g., searching for a command name like `ESC GS t`).
- **Per-page files:** `spec/docs/starprnt_cm_en-{page}.html` — one file per page. Read these for detailed command info once you know the page number.
- **PNG images:** `spec/docs/starprnt_cm_en-{page}_1.png` — page renders. These don't display well programmatically; prefer the HTML files.

**Workflow for finding a command:**
1. Grep `starprnt_cm_ens.html` for the command name or escape sequence
2. Note the page number from the matching file context
3. Read the per-page HTML file for full details (parameter ranges, model variations, etc.)
4. Cross-reference with the outline to confirm the section number

### Documentation Format for Protocol Code

```rust
/// Set text alignment.
///
/// **Command:** ESC GS a n
/// **Spec Reference:** Section 2.3.4 "Horizontal Direction Printing Position", page 42
///
/// # Parameters
/// - `n = 0`: Left align
/// - `n = 1`: Center align
/// - `n = 2`: Right align
pub fn alignment(align: Alignment) -> Vec<u8> {
    vec![ESC, GS, b'a', align as u8]
}
```

### Key Spec Sections

| Section | Topic | Page |
|---------|-------|------|
| 2.3.1 | Font Style and Character Set (incl. code pages) | 21-29 |
| 2.3.2 | Kanji Characters | 30-32 |
| 2.3.3 | Print Mode | 33-41 |
| 2.3.4 | Horizontal Direction Printing Position | 42-44 |
| 2.3.5 | Line Spacing | 45-47 |
| 2.3.9 | Cutter Control | 51 |
| 2.3.12 | Bit Image Graphics | 59-65 |
| 2.3.13 | Logo (NV Graphics) | 66-79 |
| 2.3.14 | Bar Code | 80-83 |
| 2.3.15 | QR Code | 84-89 |
| 2.3.16 | PDF417 | 90-93 |
| 2.3.18 | Initialization Command | 102-103 |

### Escape Sequence Constants

```rust
pub const ESC: u8 = 0x1B;  // Escape - command prefix
pub const GS: u8 = 0x1D;   // Group Separator - extended commands
pub const RS: u8 = 0x1E;   // Record Separator - configuration
```

## Hardware: TSP650II

- **Paper width:** 80mm (72mm printable)
- **Resolution:** 203 DPI (576 dots across)
- **Band height:** 24 rows (fixed for StarPRNT)
- **Interface:** Bluetooth RFCOMM at `/dev/rfcomm0`

## Graphics Modes

**Band Mode (ESC k):** 24-row chunks, streaming-friendly
**Raster Mode (ESC GS S):** Arbitrary height, more flexible

Both use Bayer 8x8 ordered dithering for grayscale conversion.

## Testing

Golden tests verify binary output consistency:
- Pattern renders stored as PNG in `tests/golden/`
- Binary command output stored as `.bin` files in `tests/golden/`
- Run `make golden` to regenerate after intentional changes

Frontend E2E tests use Playwright (`frontend/e2e/`):
- Canvas overlay tests: hover/select, drag, resize, content bounds (`canvas-overlay.spec.ts`)
- Run `make test-e2e` (auto-builds frontend, installs Chromium, starts server on :8090)
- Run `cd frontend && npm run test:headed` for headed mode (browser visible)
- Playwright config: `frontend/playwright.config.ts`

## Code Conventions

- **Prefer Document** over direct protocol access for new code
- Use struct literals with `..Default::default()` for component construction
- Return `Vec<u8>` for command bytes in protocol module
- Chunk large Bluetooth writes (4096 bytes, 2ms delay)
- All coordinates in dots unless otherwise noted

## TODOs

- [x] Implement NV graphics commands (`Op::NvStore`, `Op::NvPrint`, `Op::NvDelete`)
- [x] Logo repository with programmatic logo generation (`src/logos/`)
  - CLI: `logo list`, `logo sync`, `logo preview`
  - Star logo at key "A1" included by default
