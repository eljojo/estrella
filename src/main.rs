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
    components::{ComponentExt, Pattern as PatternComponent, Receipt},
    logos,
    preview,
    printer::PrinterConfig,
    protocol::{commands, nv_graphics},
    receipt,
    render::dither,
    render::patterns,
    render::weave::{BlendCurve, Weave},
    server,
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
        /// Pattern or receipt to print (use "all" to print everything, omit to see available options)
        pattern: Option<String>,

        /// List available patterns
        #[arg(long)]
        list: bool,

        /// Output to PNG file instead of printing
        #[arg(long, value_name = "FILE")]
        png: Option<PathBuf>,

        /// Printer device path
        #[arg(long, default_value = "/dev/rfcomm0")]
        device: String,

        /// Pattern height in rows (defaults to pattern's recommended height, or 500)
        #[arg(long)]
        height: Option<usize>,

        /// Pattern length in millimeters (e.g., "15mm" or "62.5mm").
        /// Overrides --height if both are specified.
        #[arg(long, value_name = "LENGTH")]
        length: Option<String>,

        /// Print width in dots
        #[arg(long, default_value = "576")]
        width: usize,

        /// Skip printing title header
        #[arg(long)]
        no_title: bool,

        /// Use band mode instead of raster mode for graphics
        #[arg(long)]
        band: bool,

        /// Render as full-page raster (no margins, 576px wide)
        /// Use this to test raster print quality vs normal text mode
        #[arg(long)]
        raster: bool,

        /// Dithering algorithm (bayer, floyd-steinberg, atkinson, jarvis)
        #[arg(long, default_value = "floyd-steinberg")]
        dither: String,

        /// Use golden (deterministic) parameters instead of randomized ones.
        /// Useful for golden tests and reproducible output.
        #[arg(long)]
        golden: bool,

        /// Set a pattern parameter (can be used multiple times).
        /// Format: name=value (e.g., --param scale=8.0 --param gamma=1.5)
        #[arg(long = "param", value_name = "NAME=VALUE")]
        params: Vec<String>,

        /// List available parameters for the specified pattern.
        #[arg(long)]
        list_params: bool,

        /// Don't print the parameter values at the bottom of the pattern.
        /// By default, randomized patterns show their parameters for reproducibility.
        #[arg(long)]
        no_params: bool,
    },

    /// Manage logos stored in printer's NV (non-volatile) memory
    Logo {
        #[command(subcommand)]
        action: LogoAction,
    },

    /// Start HTTP server for web-based printing
    Serve {
        /// Address and port to bind to
        #[arg(long, default_value = "0.0.0.0:8080")]
        listen: String,

        /// Printer device path
        #[arg(long, default_value = "/dev/rfcomm0")]
        device: String,
    },

    /// Blend multiple patterns together with crossfade transitions (like a DJ mix)
    Weave {
        /// Patterns to blend together (e.g., riley mycelium plasma waves)
        #[arg(required = true)]
        patterns: Vec<String>,

        /// Total length in millimeters (e.g., "500mm")
        #[arg(long, value_name = "LENGTH", default_value = "200mm")]
        length: String,

        /// Crossfade transition length in millimeters (e.g., "30mm")
        #[arg(long, value_name = "LENGTH", default_value = "30mm")]
        crossfade: String,

        /// Blend curve: linear, smooth, ease-in, ease-out
        #[arg(long, default_value = "smooth")]
        curve: String,

        /// Output to PNG file instead of printing
        #[arg(long, value_name = "FILE")]
        png: Option<PathBuf>,

        /// Printer device path
        #[arg(long, default_value = "/dev/rfcomm0")]
        device: String,

        /// Print width in dots
        #[arg(long, default_value = "576")]
        width: usize,

        /// Use golden (deterministic) parameters instead of randomized ones
        #[arg(long)]
        golden: bool,

        /// Dithering algorithm (bayer, floyd-steinberg, atkinson, jarvis)
        #[arg(long, default_value = "floyd-steinberg")]
        dither: String,
    },
}

#[derive(Subcommand, Debug)]
enum LogoAction {
    /// List all logos in the registry
    List,

    /// Sync registry logos to the printer's NV memory
    Sync {
        /// Printer device path
        #[arg(long, default_value = "/dev/rfcomm0")]
        device: String,

        /// Only sync a specific logo by key
        #[arg(long)]
        key: Option<String>,
    },

    /// Preview a registry logo as PNG
    Preview {
        /// Logo key to preview
        key: String,

        /// Output PNG file
        #[arg(long, value_name = "FILE")]
        png: PathBuf,

        /// Scale factor (1 or 2)
        #[arg(long, default_value = "1")]
        scale: u8,
    },

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
            png,
            device,
            height,
            length,
            width,
            no_title,
            band,
            raster,
            dither,
            golden,
            params,
            list_params,
            no_params,
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
                println!("\nSpecial:");
                println!("  all  - Print all patterns and receipts");
                return Ok(());
            }

            let name = pattern.as_deref().unwrap();

            // Handle --list-params: show available parameters for pattern
            if list_params {
                let pattern_impl = patterns::by_name_golden(name).ok_or_else(|| {
                    EstrellaError::Pattern(format!(
                        "Unknown pattern '{}'. Run without arguments to see available options.",
                        name
                    ))
                })?;

                let params_list = pattern_impl.list_params();
                if params_list.is_empty() {
                    println!("Pattern '{}' has no configurable parameters.", name);
                } else {
                    println!("Parameters for '{}':", name);
                    for (param_name, current_value) in params_list {
                        println!("  {} = {}", param_name, current_value);
                    }
                }
                return Ok(());
            }

            // Handle "all" - print all patterns and receipts
            if name == "all" {
                println!("Printing all patterns and receipts...\n");

                // Print all receipts first
                for receipt_name in receipt::list_receipts() {
                    println!("Printing receipt: {}", receipt_name);
                    let receipt_data = receipt::by_name(receipt_name).unwrap();
                    print_raw_to_device(&device, &receipt_data)?;
                }

                // Then print all patterns
                for pattern_name in patterns::list_patterns() {
                    println!("Printing pattern: {}", pattern_name);

                    // Get pattern impl - randomized by default unless --golden
                    let pattern_impl = if golden {
                        patterns::by_name_golden(pattern_name).unwrap()
                    } else {
                        patterns::by_name_random(pattern_name).unwrap()
                    };

                    let (default_width, default_height) = pattern_impl.default_dimensions();
                    let pattern_width = if width != 576 { width } else { default_width };
                    let pattern_height = if let Some(ref len) = length {
                        parse_length_mm(len)?
                    } else {
                        height.unwrap_or(default_height)
                    };

                    // Parse dithering algorithm
                    let dither_algo = match dither.to_lowercase().as_str() {
                        "bayer" => dither::DitheringAlgorithm::Bayer,
                        "floyd-steinberg" | "floyd_steinberg" | "fs" => {
                            dither::DitheringAlgorithm::FloydSteinberg
                        }
                        "atkinson" => dither::DitheringAlgorithm::Atkinson,
                        "jarvis" | "jjn" => dither::DitheringAlgorithm::Jarvis,
                        _ => {
                            return Err(EstrellaError::Pattern(format!(
                                "Unknown dithering algorithm '{}'. Use 'bayer', 'floyd-steinberg', 'atkinson', or 'jarvis'",
                                dither
                            )));
                        }
                    };

                    let mut pattern = PatternComponent::from_impl(pattern_impl, pattern_height)
                        .width(pattern_width)
                        .dithering(dither_algo);
                    if !no_title {
                        pattern = pattern.with_title();
                    }
                    if band {
                        pattern = pattern.band_mode();
                    }
                    if !no_params && !golden {
                        pattern = pattern.show_params();
                    }

                    let print_data = Receipt::new().child(pattern).cut().build();
                    print_raw_to_device(&device, &print_data)?;
                }

                println!("\nAll patterns and receipts printed successfully!");
                return Ok(());
            }

            // Check if it's a receipt template
            if receipt::is_receipt(name) {
                if raster {
                    // Raster mode: render as full-page raster (no margins)
                    return print_as_raster(name, png.as_ref(), &device);
                }

                if let Some(png_path) = png {
                    // Render receipt to PNG preview
                    println!("Generating {} receipt preview...", name);
                    let program = receipt::program_by_name(name).unwrap();
                    let png_bytes = program.to_preview_png().map_err(|e| {
                        EstrellaError::Image(format!("Failed to render preview: {}", e))
                    })?;
                    std::fs::write(&png_path, &png_bytes).map_err(|e| {
                        EstrellaError::Image(format!("Failed to write PNG: {}", e))
                    })?;
                    println!("Saved to {}", png_path.display());
                    return Ok(());
                }

                println!("Printing {} receipt...", name);
                let receipt_data = receipt::by_name(name).unwrap();
                print_raw_to_device(&device, &receipt_data)?;
                println!("Printed successfully!");
                return Ok(());
            }

            // It's a visual pattern
            // Get pattern impl - randomized by default unless --golden
            let mut pattern_impl = if golden {
                patterns::by_name_golden(name).ok_or_else(|| {
                    EstrellaError::Pattern(format!(
                        "Unknown pattern or receipt '{}'. Run without arguments to see available options.",
                        name
                    ))
                })?
            } else {
                patterns::by_name_random(name).ok_or_else(|| {
                    EstrellaError::Pattern(format!(
                        "Unknown pattern or receipt '{}'. Run without arguments to see available options.",
                        name
                    ))
                })?
            };

            // Apply any --param overrides
            for param_str in &params {
                let parts: Vec<&str> = param_str.splitn(2, '=').collect();
                if parts.len() != 2 {
                    return Err(EstrellaError::Pattern(format!(
                        "Invalid param format '{}'. Use name=value (e.g., --param scale=8.0)",
                        param_str
                    )));
                }
                pattern_impl.set_param(parts[0], parts[1]).map_err(|e| {
                    EstrellaError::Pattern(e)
                })?;
            }

            // Use pattern's default dimensions if user didn't specify
            let (default_width, default_height) = pattern_impl.default_dimensions();
            let width = if width != 576 { width } else { default_width };
            let height = if let Some(ref len) = length {
                parse_length_mm(len)?
            } else {
                height.unwrap_or(default_height)
            };

            let params_desc = pattern_impl.params_description();
            if !params_desc.is_empty() && !golden {
                println!("Generating {} pattern ({}x{}) with params: {}...", name, width, height, params_desc);
            } else {
                println!("Generating {} pattern ({}x{})...", name, width, height);
            }

            // Parse dithering algorithm
            let dither_algo = match dither.to_lowercase().as_str() {
                "bayer" => dither::DitheringAlgorithm::Bayer,
                "floyd-steinberg" | "floyd_steinberg" | "fs" => {
                    dither::DitheringAlgorithm::FloydSteinberg
                }
                "atkinson" => dither::DitheringAlgorithm::Atkinson,
                "jarvis" | "jjn" => dither::DitheringAlgorithm::Jarvis,
                _ => {
                    return Err(EstrellaError::Pattern(format!(
                        "Unknown dithering algorithm '{}'. Use 'bayer', 'floyd-steinberg', 'atkinson', or 'jarvis'",
                        dither
                    )));
                }
            };

            // Build pattern component from the impl
            let mut pattern = PatternComponent::from_impl(pattern_impl, height)
                .width(width)
                .dithering(dither_algo);
            if !no_title {
                pattern = pattern.with_title();
            }
            if band {
                pattern = pattern.band_mode();
            }
            if !no_params && !golden {
                pattern = pattern.show_params();
            }

            // Output to PNG or printer
            if let Some(png_path) = png {
                let program = Receipt::new().child(pattern).cut().compile();
                let png_bytes = program.to_preview_png().map_err(|e| {
                    EstrellaError::Image(format!("Failed to render preview: {}", e))
                })?;
                std::fs::write(&png_path, &png_bytes).map_err(|e| {
                    EstrellaError::Image(format!("Failed to write PNG: {}", e))
                })?;
                println!("Saved to {}", png_path.display());
            } else {
                let print_data = Receipt::new().child(pattern).cut().build();
                print_raw_to_device(&device, &print_data)?;
                println!("Printed successfully!");
            }
        }

        Commands::Logo { action } => match action {
            LogoAction::List => {
                logo_list()?;
            }
            LogoAction::Sync { device, key } => {
                logo_sync(&device, key.as_deref())?;
            }
            LogoAction::Preview { key, png, scale } => {
                logo_preview(&key, &png, scale)?;
            }
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

        Commands::Serve { listen, device } => {
            let config = server::ServerConfig {
                device_path: device,
                listen_addr: listen,
            };

            // Create tokio runtime and run the server
            tokio::runtime::Runtime::new()
                .map_err(|e| EstrellaError::Transport(format!("Failed to create tokio runtime: {}", e)))?
                .block_on(server::serve(config))?;
        }

        Commands::Weave {
            patterns: pattern_names,
            length,
            crossfade,
            curve,
            png,
            device,
            width,
            golden,
            dither,
        } => {
            weave_patterns(
                &pattern_names,
                &length,
                &crossfade,
                &curve,
                png.as_ref(),
                &device,
                width,
                golden,
                &dither,
            )?;
        }
    }

    Ok(())
}

/// Print raw command data to the printer device
fn print_raw_to_device(device: &str, data: &[u8]) -> Result<(), EstrellaError> {
    let mut transport = BluetoothTransport::open(device)?;
    transport.write_all(data)?;
    Ok(())
}

/// Parse a length string like "15mm" or "62.5mm" and convert to height in dots.
fn parse_length_mm(length: &str) -> Result<usize, EstrellaError> {
    let length = length.trim().to_lowercase();
    let mm_str = length.strip_suffix("mm").ok_or_else(|| {
        EstrellaError::Pattern(format!(
            "Invalid length format '{}'. Use format like '15mm' or '62.5mm'",
            length
        ))
    })?;
    let mm: f32 = mm_str.parse().map_err(|_| {
        EstrellaError::Pattern(format!(
            "Invalid length value '{}'. Use format like '15mm' or '62.5mm'",
            length
        ))
    })?;
    if mm <= 0.0 {
        return Err(EstrellaError::Pattern("Length must be positive".to_string()));
    }
    Ok(PrinterConfig::TSP650II.mm_to_dots(mm) as usize)
}

/// Print a receipt as a full-page raster (no margins, 576px wide).
///
/// This renders the receipt to a pixel buffer and prints it as a single raster image.
/// Useful for testing raster quality vs normal text mode printing.
fn print_as_raster(name: &str, png_path: Option<&PathBuf>, device: &str) -> Result<(), EstrellaError> {
    use image::{GrayImage, Luma};

    println!("Rendering {} as raster (576px, no margins)...", name);

    // Get the program for this receipt
    let program = receipt::program_by_name(name).ok_or_else(|| {
        EstrellaError::Pattern(format!("Unknown receipt '{}'", name))
    })?;

    // Render to raw pixel buffer (no margins)
    let raw = preview::render_raw(&program).map_err(|e| {
        EstrellaError::Image(format!("Failed to render: {}", e))
    })?;

    println!("Rendered {}x{} pixels ({} bytes)", raw.width, raw.height, raw.data.len());

    // Save to PNG if requested
    if let Some(png_path) = png_path {
        let width_bytes = raw.width.div_ceil(8);
        let mut img = GrayImage::new(raw.width as u32, raw.height as u32);

        for y in 0..raw.height {
            for x in 0..raw.width {
                let byte_idx = y * width_bytes + x / 8;
                let bit_idx = 7 - (x % 8);
                let is_black = (raw.data[byte_idx] >> bit_idx) & 1 == 1;
                let color = if is_black { 0u8 } else { 255u8 };
                img.put_pixel(x as u32, y as u32, Luma([color]));
            }
        }

        img.save(png_path).map_err(|e| {
            EstrellaError::Image(format!("Failed to save PNG: {}", e))
        })?;
        println!("Saved raster preview to {}", png_path.display());
    }

    // Print to device if no PNG-only mode
    if png_path.is_none() || std::env::args().any(|a| a == "--print") {
        println!("Printing as raster ({} rows)...", raw.height);

        // Build IR program - codegen handles chunking automatically
        use estrella::ir::{Op, Program};

        let mut program = Program::new();
        program.push(Op::Init);
        program.push(Op::Raster {
            width: raw.width as u16,
            height: raw.height as u16,
            data: raw.data.clone(),
        });
        program.push(Op::Feed { units: 24 }); // 6mm
        program.push(Op::Cut { partial: false });

        // Compile to bytes (chunking happens here)
        let print_data = program.to_bytes();
        print_raw_to_device(device, &print_data)?;

        println!("Printed successfully!");
    }

    Ok(())
}

// ============================================================================
// LOGO COMMANDS
// ============================================================================

/// List all logos in the registry.
fn logo_list() -> Result<(), EstrellaError> {
    let all_logos = logos::all();
    if all_logos.is_empty() {
        println!("No logos registered.");
    } else {
        println!("Registered logos:");
        for logo in all_logos {
            let raster = logo.raster();
            println!("  {} - {} ({}x{})", logo.key, logo.name, raster.width, raster.height);
        }
    }
    Ok(())
}

/// Sync registry logos to the printer's NV memory.
fn logo_sync(device: &str, key: Option<&str>) -> Result<(), EstrellaError> {
    let logos_to_sync: Vec<_> = if let Some(k) = key {
        logos::by_key(k)
            .map(|l| vec![l])
            .ok_or_else(|| {
                EstrellaError::Pattern(format!("Unknown logo key '{}'. Run 'logo list' to see available logos.", k))
            })?
    } else {
        logos::all().iter().collect()
    };

    if logos_to_sync.is_empty() {
        println!("No logos to sync.");
        return Ok(());
    }

    for logo in logos_to_sync {
        let raster = logo.raster();
        let cmd = nv_graphics::define(logo.key, raster.width, raster.height, &raster.data)
            .ok_or_else(|| {
                EstrellaError::Pattern(format!("Failed to generate NV store command for '{}'", logo.key))
            })?;

        println!("Syncing '{}' ({}) - {}x{} ({} bytes)...",
            logo.name, logo.key, raster.width, raster.height, cmd.len());

        let mut data = commands::init();
        data.extend(cmd);
        print_raw_to_device(device, &data)?;
    }

    println!("Sync complete!");
    Ok(())
}

/// Preview a registry logo as PNG.
fn logo_preview(key: &str, png_path: &PathBuf, scale: u8) -> Result<(), EstrellaError> {
    use image::{GrayImage, Luma};

    let logo = logos::by_key(key).ok_or_else(|| {
        EstrellaError::Pattern(format!("Unknown logo key '{}'. Run 'logo list' to see available logos.", key))
    })?;

    let raster = logo.raster();
    let scale = scale.clamp(1, 2) as usize;

    let width = raster.width as usize * scale;
    let height = raster.height as usize * scale;
    let src_width_bytes = (raster.width as usize).div_ceil(8);

    let mut img = GrayImage::new(width as u32, height as u32);

    for sy in 0..raster.height as usize {
        for sx in 0..raster.width as usize {
            let byte_idx = sy * src_width_bytes + sx / 8;
            let bit_idx = 7 - (sx % 8);
            let pixel_on = (raster.data[byte_idx] >> bit_idx) & 1 == 1;
            let color = if pixel_on { 0u8 } else { 255u8 };

            for dy in 0..scale {
                for dx in 0..scale {
                    let px = sx * scale + dx;
                    let py = sy * scale + dy;
                    img.put_pixel(px as u32, py as u32, Luma([color]));
                }
            }
        }
    }

    img.save(png_path).map_err(|e| {
        EstrellaError::Image(format!("Failed to save PNG: {}", e))
    })?;

    println!("Saved {} ({}) preview to {}", logo.name, logo.key, png_path.display());
    Ok(())
}

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

// ============================================================================
// WEAVE COMMAND
// ============================================================================

/// Blend multiple patterns together with crossfade transitions.
fn weave_patterns(
    pattern_names: &[String],
    length: &str,
    crossfade: &str,
    curve: &str,
    png_path: Option<&PathBuf>,
    device: &str,
    width: usize,
    golden: bool,
    dither_name: &str,
) -> Result<(), EstrellaError> {
    use image::{GrayImage, Luma};

    if pattern_names.len() < 2 {
        return Err(EstrellaError::Pattern(
            "Weave requires at least 2 patterns".to_string(),
        ));
    }

    // Parse length and crossfade
    let height = parse_length_mm(length)?;
    let crossfade_pixels = parse_length_mm(crossfade)?;

    // Parse blend curve
    let blend_curve = BlendCurve::from_str(curve).ok_or_else(|| {
        EstrellaError::Pattern(format!(
            "Unknown blend curve '{}'. Use: linear, smooth, ease-in, ease-out",
            curve
        ))
    })?;

    // Parse dithering algorithm
    let dither_algo = match dither_name.to_lowercase().as_str() {
        "bayer" => dither::DitheringAlgorithm::Bayer,
        "floyd-steinberg" | "floyd_steinberg" | "fs" => dither::DitheringAlgorithm::FloydSteinberg,
        "atkinson" => dither::DitheringAlgorithm::Atkinson,
        "jarvis" | "jjn" => dither::DitheringAlgorithm::Jarvis,
        _ => {
            return Err(EstrellaError::Pattern(format!(
                "Unknown dithering algorithm '{}'. Use 'bayer', 'floyd-steinberg', 'atkinson', or 'jarvis'",
                dither_name
            )));
        }
    };

    // Load patterns
    let mut pattern_impls: Vec<Box<dyn patterns::Pattern>> = Vec::new();
    for name in pattern_names {
        let pattern = if golden {
            patterns::by_name_golden(name)
        } else {
            patterns::by_name_random(name)
        }
        .ok_or_else(|| {
            EstrellaError::Pattern(format!(
                "Unknown pattern '{}'. Run 'estrella print' to see available patterns.",
                name
            ))
        })?;
        pattern_impls.push(pattern);
    }

    // Create the weave
    let pattern_refs: Vec<&dyn patterns::Pattern> =
        pattern_impls.iter().map(|p| p.as_ref()).collect();
    let weave = Weave::new(pattern_refs)
        .crossfade_pixels(crossfade_pixels)
        .curve(blend_curve);

    println!(
        "Weaving {} patterns ({}x{}) with {}px crossfade, {} curve...",
        pattern_names.len(),
        width,
        height,
        crossfade_pixels,
        curve
    );
    println!("  Patterns: {}", pattern_names.join(" -> "));

    // Render using the dithering module's generate_raster
    let raster_data = dither::generate_raster(
        width,
        height,
        |x, y, w, h| weave.intensity(x, y, w, h),
        dither_algo,
    );
    let width_bytes = width.div_ceil(8);

    // Output to PNG or printer
    if let Some(png_path) = png_path {
        let mut img = GrayImage::new(width as u32, height as u32);

        for y in 0..height {
            for x in 0..width {
                let byte_idx = y * width_bytes + x / 8;
                let bit_idx = 7 - (x % 8);
                let is_black = (raster_data[byte_idx] >> bit_idx) & 1 == 1;
                let color = if is_black { 0u8 } else { 255u8 };
                img.put_pixel(x as u32, y as u32, Luma([color]));
            }
        }

        img.save(png_path).map_err(|e| {
            EstrellaError::Image(format!("Failed to save PNG: {}", e))
        })?;
        println!("Saved to {}", png_path.display());
    } else {
        // Print to device
        use estrella::ir::{Op, Program};

        let mut program = Program::new();
        program.push(Op::Init);
        program.push(Op::Raster {
            width: width as u16,
            height: height as u16,
            data: raster_data,
        });
        program.push(Op::Feed { units: 24 }); // 6mm
        program.push(Op::Cut { partial: false });

        let print_data = program.to_bytes();
        print_raw_to_device(device, &print_data)?;
        println!("Printed successfully!");
    }

    Ok(())
}
