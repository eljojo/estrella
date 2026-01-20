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
    EstrellaError,
    components::{Component, ComponentExt, Image, Receipt, Spacer, Text},
    ir::Op,
    receipt,
    render::patterns,
    transport::BluetoothTransport,
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

        /// Pattern height in rows (defaults to pattern's recommended height, or 500)
        #[arg(long)]
        height: Option<usize>,

        /// Print width in dots
        #[arg(long, default_value = "576")]
        width: usize,

        /// Skip printing title header
        #[arg(long)]
        no_title: bool,

        /// Use band mode instead of raster mode for graphics
        #[arg(long)]
        band: bool,
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
            band,
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

            // Use pattern's default dimensions if user didn't specify
            let (default_width, default_height) = pattern_impl.default_dimensions();
            let width = if width != 576 { width } else { default_width };
            let height = height.unwrap_or(default_height);

            println!("Generating {} pattern ({}x{})...", name, width, height);

            // Render pattern
            let raster_data = pattern_impl.render(width, height);

            // Output to PNG or printer
            if let Some(png_path) = png {
                save_png(&png_path, width, height, &raster_data)?;
                println!("Saved to {}", png_path.display());
            } else {
                print_pattern_to_device(
                    &device,
                    name,
                    width as u16,
                    height as u16,
                    &raster_data,
                    !no_title,
                    band,
                )?;
                println!("Printed successfully!");
            }
        }
    }

    Ok(())
}

/// Save raster data as a PNG image
fn save_png(path: &PathBuf, width: usize, height: usize, data: &[u8]) -> Result<(), EstrellaError> {
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

/// A component for pattern title headers
struct PatternTitle {
    name: String,
}

impl PatternTitle {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_uppercase(),
        }
    }
}

impl Component for PatternTitle {
    fn emit(&self, ops: &mut Vec<Op>) {
        // Centered horizontal rule
        Text::new("================================")
            .center()
            .emit(ops);

        // Pattern name in bold, double height
        Text::new(&self.name)
            .center()
            .bold()
            .double_height()
            .emit(ops);

        // Horizontal rule
        Text::new("================================")
            .center()
            .emit(ops);

        // Small spacing before pattern
        Spacer::mm(2.0).emit(ops);

        // Reset alignment for pattern (left)
        ops.push(Op::SetAlign(estrella::protocol::text::Alignment::Left));
    }
}

/// Print pattern with optional title to the printer device
///
/// Uses raster mode (ESC GS S) by default, or band mode (ESC k) if requested.
fn print_pattern_to_device(
    device: &str,
    name: &str,
    width: u16,
    height: u16,
    data: &[u8],
    with_title: bool,
    use_band_mode: bool,
) -> Result<(), EstrellaError> {
    // Build print sequence using components
    let mut receipt = Receipt::new();

    // Add title if requested
    if with_title {
        receipt = receipt.child(PatternTitle::new(name));
    }

    // Add the pattern image
    let image = if use_band_mode {
        Image::from_raster(width, height, data.to_vec()).band_mode()
    } else {
        Image::from_raster(width, height, data.to_vec()).raster_mode()
    };
    receipt = receipt.child(image).cut();

    // Build and send to printer
    let print_data = receipt.build();
    print_raw_to_device(device, &print_data)
}
