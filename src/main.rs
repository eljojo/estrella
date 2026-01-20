//! # Estrella CLI
//!
//! Command-line interface for thermal receipt printing.
//!
//! ## Usage
//!
//! ```bash
//! # List available patterns and receipts
//! estrella print
//!
//! # Print a visual pattern
//! estrella print ripple
//!
//! # Print with custom height
//! estrella print --height 1000 waves
//!
//! # Save pattern as PNG (patterns only)
//! estrella print --png output.png ripple
//!
//! # Print a demo receipt
//! estrella print receipt
//!
//! # Print full receipt with barcodes
//! estrella print receipt-full
//! ```

use clap::{Parser, Subcommand};
use std::path::PathBuf;

use estrella::{
    printer::PrinterConfig,
    protocol::{commands, graphics, text},
    receipt,
    render::patterns,
    transport::BluetoothTransport,
    EstrellaError,
};

/// Estrella - Thermal receipt printer utility
#[derive(Parser, Debug)]
#[command(name = "estrella")]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Print a pattern to the thermal printer
    Print {
        /// Pattern or receipt to print (omit to see available options)
        pattern: Option<String>,

        /// List available patterns
        #[arg(long)]
        list: bool,

        /// Enable kitchensink mode (demo patterns)
        #[arg(long)]
        kitchensink: bool,

        /// Output to PNG file instead of printing
        #[arg(long, value_name = "FILE")]
        png: Option<PathBuf>,

        /// Printer device path
        #[arg(long, default_value = "/dev/rfcomm0")]
        device: String,

        /// Pattern height in rows
        #[arg(long, default_value = "500")]
        height: usize,

        /// Print width in dots
        #[arg(long, default_value = "576")]
        width: usize,

        /// Skip printing title header
        #[arg(long)]
        no_title: bool,
    },
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<(), EstrellaError> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Print {
            pattern,
            list,
            kitchensink: _,
            png,
            device,
            height,
            width,
            no_title,
        } => {
            // List patterns if --list flag or no pattern specified
            if list || pattern.is_none() {
                println!("Available patterns:");
                for name in patterns::list_patterns() {
                    println!("  {}", name);
                }
                println!("\nAvailable receipts:");
                for name in receipt::list_receipts() {
                    println!("  {}", name);
                }
                return Ok(());
            }

            let name = pattern.as_deref().unwrap();

            // Check if it's a receipt template
            if receipt::is_receipt(name) {
                if png.is_some() {
                    return Err(EstrellaError::Pattern(
                        "PNG preview is not available for receipts (text-based output)".to_string(),
                    ));
                }

                println!("Printing {} receipt...", name);
                let receipt_data = receipt::by_name(name).unwrap();
                print_raw_to_device(&device, &receipt_data)?;
                println!("Printed successfully!");
                return Ok(());
            }

            // It's a visual pattern
            let pattern_impl = patterns::by_name(name).ok_or_else(|| {
                EstrellaError::Pattern(format!(
                    "Unknown pattern or receipt '{}'. Run without arguments to see available options.",
                    name
                ))
            })?;

            println!("Generating {} pattern ({}x{})...", name, width, height);

            // Render pattern
            let raster_data = pattern_impl.render(width, height);

            // Output to PNG or printer
            if let Some(png_path) = png {
                save_png(&png_path, width, height, &raster_data)?;
                println!("Saved to {}", png_path.display());
            } else {
                print_pattern_to_device(&device, name, width as u16, height as u16, &raster_data, !no_title)?;
                println!("Printed successfully!");
            }
        }
    }

    Ok(())
}

/// Save raster data as a PNG image
fn save_png(
    path: &PathBuf,
    width: usize,
    height: usize,
    data: &[u8],
) -> Result<(), EstrellaError> {
    use image::{GrayImage, Luma};

    let mut img = GrayImage::new(width as u32, height as u32);
    let width_bytes = width.div_ceil(8);

    for y in 0..height {
        for x in 0..width {
            let byte_idx = y * width_bytes + x / 8;
            let bit_idx = 7 - (x % 8);
            let is_black = (data[byte_idx] >> bit_idx) & 1 == 1;

            let color = if is_black { 0u8 } else { 255u8 };
            img.put_pixel(x as u32, y as u32, Luma([color]));
        }
    }

    img.save(path)
        .map_err(|e| EstrellaError::Image(format!("Failed to save PNG: {}", e)))?;

    Ok(())
}

/// Print raw command data to the printer device
fn print_raw_to_device(device: &str, data: &[u8]) -> Result<(), EstrellaError> {
    let mut transport = BluetoothTransport::open(device)?;
    transport.write_all(data)?;
    Ok(())
}

/// Generate a title header for a pattern
fn make_title(name: &str) -> Vec<u8> {
    let mut data = Vec::new();

    // Center align
    data.extend(text::align_center());

    // Horizontal rule
    data.extend(b"================================\n");

    // Pattern name in bold, double height
    data.extend(text::bold_on());
    data.extend(text::double_height_on());
    data.extend(name.to_uppercase().as_bytes());
    data.push(0x0A); // LF
    data.extend(text::double_height_off());
    data.extend(text::bold_off());

    // Horizontal rule
    data.extend(b"================================\n");

    // Small spacing before pattern
    data.extend(commands::feed_mm(2.0));

    // Reset alignment for pattern
    data.extend(text::align_left());

    data
}

/// Print pattern with optional title to the printer device
///
/// Uses band mode (ESC k) for "sick" pattern to match Python implementation,
/// raster mode (ESC GS S) for other patterns.
fn print_pattern_to_device(
    device: &str,
    name: &str,
    width: u16,
    height: u16,
    data: &[u8],
    with_title: bool,
) -> Result<(), EstrellaError> {
    // Build print sequence
    let mut print_data = Vec::new();

    // Initialize printer
    print_data.extend(commands::init());

    // Add title if requested
    if with_title {
        print_data.extend(make_title(name));
    }

    // Use band mode for "sick" pattern (matches Python sick.py behavior)
    // Use raster mode for "sick-raster" to test if TTY fixes help
    // Band mode: ESC k n1 0 + 24 rows of data + ESC J 12 feed
    // Spec: StarPRNT Command Spec Rev 4.10, Section 2.3.12, page 61
    let name_lower = name.to_lowercase();
    if name_lower == "sick" {
        print_band_mode(&mut print_data, width, height, data);
    } else {
        print_raster_mode(&mut print_data, width, height, data);
    }

    // Cut paper
    print_data.extend(commands::cut_full_feed());

    // Send to printer
    print_raw_to_device(device, &print_data)
}

/// Print using raster mode (ESC GS S)
/// Spec: StarPRNT Command Spec Rev 4.10, Section 2.3.12, page 63
fn print_raster_mode(print_data: &mut Vec<u8>, width: u16, height: u16, data: &[u8]) {
    let config = PrinterConfig::TSP650II;
    let width_bytes = (width as usize).div_ceil(8);
    let chunk_rows = config.max_chunk_rows as usize;

    let mut row_offset = 0;
    while row_offset < height as usize {
        let chunk_height = (height as usize - row_offset).min(chunk_rows);
        let byte_start = row_offset * width_bytes;
        let byte_end = (row_offset + chunk_height) * width_bytes;
        let chunk_data = &data[byte_start..byte_end];

        print_data.extend(graphics::raster(width, chunk_height as u16, chunk_data));

        row_offset += chunk_height;
    }
}

/// Print using band mode (ESC k) - 24 rows at a time with feed after each band
/// Spec: StarPRNT Command Spec Rev 4.10, Section 2.3.12, page 61
///
/// This mode is more reliable for some patterns as it matches the original
/// Python implementation of sick.py.
fn print_band_mode(print_data: &mut Vec<u8>, width: u16, height: u16, data: &[u8]) {
    const BAND_HEIGHT: usize = 24;
    let width_bytes = (width as usize).div_ceil(8);

    let mut row_offset = 0;
    while row_offset < height as usize {
        let band_rows = (height as usize - row_offset).min(BAND_HEIGHT);
        let byte_start = row_offset * width_bytes;
        let byte_end = (row_offset + band_rows) * width_bytes;
        let band_data = &data[byte_start..byte_end];

        // ESC k n1 n2 (n2 is always 0)
        // n1 = width in bytes (72 for 576 dots)
        // Spec: k = (n1 + n2 × 256) × 24 bytes of data expected
        print_data.extend(graphics::band(width_bytes as u8, band_data));

        // ESC J n - feed n/4 mm (n=12 → 3mm, matches 24 dots at ~8 dots/mm)
        // Spec: StarPRNT Command Spec Rev 4.10, Section 2.2.1
        print_data.extend(commands::feed_units(12));

        row_offset += band_rows;
    }
}
