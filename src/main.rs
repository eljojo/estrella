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
    protocol::{commands, graphics},
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
        /// Pattern to print (omit to see available patterns)
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
                print_raster_to_device(&device, width as u16, height as u16, &raster_data)?;
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

/// Print raster data to the printer device
fn print_raster_to_device(
    device: &str,
    width: u16,
    height: u16,
    data: &[u8],
) -> Result<(), EstrellaError> {
    let config = PrinterConfig::TSP650II;

    // Build print sequence
    let mut print_data = Vec::new();

    // Initialize printer
    print_data.extend(commands::init());

    // Send raster in chunks to avoid Bluetooth buffer overflow
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

    // Cut paper
    print_data.extend(commands::cut_full_feed());

    // Send to printer
    print_raw_to_device(device, &print_data)
}
