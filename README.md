estrella
========

A print server for the [StarPRNT](https://starmicronics.com/support/download/starprnt-command-specifications/) protocol, targetting the [Star Micronics TSP650II](https://star-m.jp/eng/products/s_print/tsp650ii/index.html) thermal printer.

## Usage

```bash
# List available patterns
estrella print

# Print a pattern to the printer
estrella print ripple

# Save as PNG instead of printing
estrella print --png output.png waves

# Custom dimensions
estrella print --height 1000 --width 576 sick
```

## Patterns

### Ripple
Radial ripples with wobble interference - creates a hypnotic expanding wave effect.

![Ripple Pattern](tests/golden/ripple_576x500.png)

### Waves
Multi-oscillator interference pattern - overlapping sine waves create complex moiré effects.

![Waves Pattern](tests/golden/waves_576x500.png)

### Sick
Four-section test pattern for calibration and visual effects:
- Plasma/Moiré interference
- Concentric rings with diagonal waves
- Topographic contour lines
- Glitch effect with scanlines

![Sick Pattern](tests/golden/sick_576x1920.png)

### Calibration
Diagnostic pattern with borders, X-shaped diagonals, and progressive-width vertical bars.

![Calibration Pattern](tests/golden/calibration_576x500.png)

## Development

```bash
make build      # Build release binary
make test       # Run all tests
make format     # Format code
make golden     # Regenerate golden test images
make help       # Show all targets
```
