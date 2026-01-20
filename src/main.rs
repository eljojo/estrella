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
//!
//! # Store a logo in printer's NV memory
//! estrella logo store --key A0 logo.png
//!
//! # Delete a stored logo
//! estrella logo delete --key A0
//! ```

use clap::{Parser, Subcommand};
use std::io::{self, Write};
use std::path::PathBuf;

use estrella::{
    EstrellaError,
    components::{Component, ComponentExt, Image, Receipt, Spacer, Text},
    ir::Op,
    protocol::{commands, nv_graphics},
    receipt,
    render::dither,
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

    /// Manage logos stored in printer's NV (non-volatile) memory
    Logo {
        #[command(subcommand)]
        action: LogoAction,
    },
}

#[derive(Subcommand, Debug)]
enum LogoAction {
    /// Store a logo image in the printer's NV memory
    Store {
        /// PNG image file to store
        image: PathBuf,

        /// 2-character key to identify the logo (e.g., "A0", "LG")
        #[arg(long, default_value = "A0")]
        key: String,

        /// Printer device path
        #[arg(long, default_value = "/dev/rfcomm0")]
        device: String,

        /// Print width in dots (image will be centered/scaled to fit)
        #[arg(long, default_value = "576")]
        width: usize,
    },

    /// Delete a logo from the printer's NV memory
    Delete {
        /// 2-character key of the logo to delete (e.g., "A0", "LG")
        #[arg(long, default_value = "A0")]
        key: String,

        /// Printer device path
        #[arg(long, default_value = "/dev/rfcomm0")]
        device: String,
    },

    /// Delete ALL logos from the printer's NV memory
    DeleteAll {
        /// Printer device path
        #[arg(long, default_value = "/dev/rfcomm0")]
        device: String,

        /// Skip confirmation prompt
        #[arg(long)]
        force: bool,
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

        Commands::Logo { action } => match action {
            LogoAction::Store {
                image,
                key,
                device,
                width,
            } => {
                logo_store(&image, &key, &device, width)?;
            }
            LogoAction::Delete { key, device } => {
                logo_delete(&key, &device)?;
            }
            LogoAction::DeleteAll { device, force } => {
                logo_delete_all(&device, force)?;
            }
        },
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

// ============================================================================
// LOGO COMMANDS
// ============================================================================

/// Store a logo image in the printer's NV memory.
fn logo_store(
    image_path: &PathBuf,
    key: &str,
    device: &str,
    target_width: usize,
) -> Result<(), EstrellaError> {
    use image::ImageReader;
    use image::GenericImageView;

    // Validate key
    if nv_graphics::validate_key(key).is_none() {
        return Err(EstrellaError::Pattern(format!(
            "Invalid key '{}'. Key must be exactly 2 printable ASCII characters (e.g., 'A0', 'LG').",
            key
        )));
    }

    // Load the image
    println!("Loading image: {}", image_path.display());
    let img = ImageReader::open(image_path)
        .map_err(|e| EstrellaError::Image(format!("Failed to open image: {}", e)))?
        .decode()
        .map_err(|e| EstrellaError::Image(format!("Failed to decode image: {}", e)))?;

    let (img_width, img_height) = img.dimensions();
    println!("Image dimensions: {}x{}", img_width, img_height);

    // Convert to grayscale
    let gray = img.to_luma8();

    // Calculate dimensions for printer
    // Scale image to fit target width while maintaining aspect ratio
    let scale = target_width as f32 / img_width as f32;
    let scaled_height = (img_height as f32 * scale).round() as u32;

    // Resize image
    let resized = image::imageops::resize(
        &gray,
        target_width as u32,
        scaled_height,
        image::imageops::FilterType::Lanczos3,
    );

    println!(
        "Scaled to {}x{} for printer",
        target_width, scaled_height
    );

    // Dither the image
    let width_bytes = target_width.div_ceil(8);
    let mut raster_data = vec![0u8; width_bytes * scaled_height as usize];

    for y in 0..scaled_height as usize {
        for x in 0..target_width {
            let pixel = resized.get_pixel(x as u32, y as u32).0[0];
            // Invert: 255 (white) -> 0.0, 0 (black) -> 1.0
            let intensity = 1.0 - (pixel as f32 / 255.0);

            // Apply dithering
            let dithered = dither::should_print(x, y, intensity);

            if dithered {
                let byte_idx = y * width_bytes + x / 8;
                let bit_idx = 7 - (x % 8);
                raster_data[byte_idx] |= 1 << bit_idx;
            }
        }
    }

    // Generate NV store command
    let store_cmd = nv_graphics::define(key, target_width as u16, scaled_height as u16, &raster_data)
        .ok_or_else(|| {
            EstrellaError::Pattern("Failed to generate NV store command".to_string())
        })?;

    // Send to printer with init
    println!("Storing logo with key '{}' ({} bytes)...", key, store_cmd.len());
    let mut data = commands::init();
    data.extend(store_cmd);

    print_raw_to_device(device, &data)?;
    println!("Logo stored successfully!");
    println!("Use 'NvLogo::new(\"{}\")' in code or print with scale: estrella logo print --key {}", key, key);

    Ok(())
}

/// Delete a logo from the printer's NV memory.
fn logo_delete(key: &str, device: &str) -> Result<(), EstrellaError> {
    // Validate key
    if nv_graphics::validate_key(key).is_none() {
        return Err(EstrellaError::Pattern(format!(
            "Invalid key '{}'. Key must be exactly 2 printable ASCII characters.",
            key
        )));
    }

    let delete_cmd = nv_graphics::erase(key).ok_or_else(|| {
        EstrellaError::Pattern("Failed to generate NV delete command".to_string())
    })?;

    println!("Deleting logo with key '{}'...", key);
    let mut data = commands::init();
    data.extend(delete_cmd);

    print_raw_to_device(device, &data)?;
    println!("Logo deleted successfully!");

    Ok(())
}

/// Delete ALL logos from the printer's NV memory.
fn logo_delete_all(device: &str, force: bool) -> Result<(), EstrellaError> {
    if !force {
        print!("WARNING: This will delete ALL stored logos. Continue? [y/N] ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();

        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Aborted.");
            return Ok(());
        }
    }

    println!("Deleting ALL logos...");
    let mut data = commands::init();
    data.extend(nv_graphics::erase_all());

    print_raw_to_device(device, &data)?;
    println!("All logos deleted successfully!");

    Ok(())
}
