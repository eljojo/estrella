//! # Estrella CLI
//!
//! Command-line interface for thermal receipt printing.
//!
//! ## Usage
//!
//! ```bash
//! # Print a ripple pattern
//! estrella print ripple
//!
//! # Print with custom height
//! estrella print --height 1000 waves
//!
//! # Save as PNG instead of printing
//! estrella print --png output.png ripple
//!
//! # List available patterns
//! estrella print --list
//! ```

use clap::{Parser, Subcommand};
use std::path::PathBuf;

use estrella::{
    printer::PrinterConfig,
    protocol::{commands, graphics},
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
        /// Pattern to print (ripple, waves, sick)
        #[arg(required_unless_present = "list")]
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
            // List patterns
            if list {
                println!("Available patterns:");
                for name in patterns::list_patterns() {
                    println!("  {}", name);
                }
                return Ok(());
            }

            // Get pattern
            let pattern_name = pattern.as_deref().unwrap_or("ripple");
            let pattern_impl = patterns::by_name(pattern_name).ok_or_else(|| {
                EstrellaError::Pattern(format!(
                    "Unknown pattern '{}'. Use --list to see available patterns.",
                    pattern_name
                ))
            })?;

            println!(
                "Generating {} pattern ({}x{})...",
                pattern_name, width, height
            );

            // Render pattern
            let raster_data = pattern_impl.render(width, height);

            // Output to PNG or printer
            if let Some(png_path) = png {
                save_png(&png_path, width, height, &raster_data)?;
                println!("Saved to {}", png_path.display());
            } else {
                print_to_device(&device, width as u16, height as u16, &raster_data)?;
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
    let width_bytes = (width + 7) / 8;

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

/// Print raster data to the printer device
fn print_to_device(
    device: &str,
    width: u16,
    height: u16,
    data: &[u8],
) -> Result<(), EstrellaError> {
    let config = PrinterConfig::TSP650II;

    // Open transport
    let mut transport = BluetoothTransport::open(device)?;

    // Build print sequence
    let mut print_data = Vec::new();

    // Initialize printer
    print_data.extend(commands::init());

    // Send raster in chunks to avoid Bluetooth buffer overflow
    let width_bytes = (width as usize + 7) / 8;
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
    transport.write_all(&print_data)?;

    Ok(())
}
