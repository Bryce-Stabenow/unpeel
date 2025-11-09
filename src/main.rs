use std::env;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read};
use std::path::{Path, PathBuf};
use png::{Decoder, Encoder, ColorType};
use rand::Rng;

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() != 2 {
        eprintln!("Usage: {} <path_to_png>", args[0]);
        std::process::exit(1);
    }
    
    let file_path = &args[1];
    let path = Path::new(file_path);
    
    // Check if file exists
    if !path.exists() {
        eprintln!("Error: File '{}' does not exist", file_path);
        std::process::exit(1);
    }
    
    // File system metadata
    println!("=== File System Metadata ===");
    if let Ok(metadata) = std::fs::metadata(path) {
        println!("File size: {} bytes", metadata.len());
        if let Ok(modified) = metadata.modified() {
            println!("Last modified: {:?}", modified);
        }
        if let Ok(created) = metadata.created() {
            println!("Created: {:?}", created);
        }
    }
    
    println!("\n=== PNG Image Metadata ===");
    
    // Open and decode PNG
    let file = match File::open(path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Error opening file: {}", e);
            std::process::exit(1);
        }
    };
    
    let reader = BufReader::new(file);
    let decoder = Decoder::new(reader);
    
    let mut reader = match decoder.read_info() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Error reading PNG: {}", e);
            std::process::exit(1);
        }
    };
    
    // Get info and clone it before reading frame (to avoid borrowing issues)
    let info = reader.info();
    let width = info.width;
    let height = info.height;
    let color_type = info.color_type;
    let bit_depth = info.bit_depth;
    let bytes_per_pixel = info.bytes_per_pixel();
    let trns = info.trns.as_ref().map(|cow| cow.to_vec());
    
    // Allocate buffer for image data
    // Calculate buffer size: width * height * bytes_per_pixel
    let buffer_size = (width as usize) * (height as usize) * bytes_per_pixel;
    let mut buf = vec![0; buffer_size];
    
    // Read image data
    match reader.next_frame(&mut buf) {
        Ok(_) => {},
        Err(e) => {
            eprintln!("Error reading image data: {}", e);
            std::process::exit(1);
        }
    }
    
    // Add randomized noise to each pixel
    add_randomized_noise(&mut buf, color_type);
    
    // Basic image information
    println!("Width: {} pixels", width);
    println!("Height: {} pixels", height);
    println!("Color type: {:?}", color_type);
    println!("Bit depth: {:?}", bit_depth);
    println!("Bytes per pixel: {}", bytes_per_pixel);
    
    // Manually parse PNG file to read chunks
    let mut file = match File::open(path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Error opening file: {}", e);
            std::process::exit(1);
        }
    };
    
    // Skip PNG signature (8 bytes)
    let mut signature = [0u8; 8];
    if let Err(_) = file.read_exact(&mut signature) {
        println!("Could not read PNG signature.");
    } else {
        // Verify PNG signature
        let png_signature = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        if signature != png_signature {
            println!("Warning: File does not have a valid PNG signature.");
        }
    }
    
    println!("\n=== Summary ===");
    println!("File: {}", file_path);
    println!("Dimensions: {}x{}", width, height);
    println!("Color format: {:?} at {:?} bits", color_type, bit_depth);
    
    // Create output file path with "-unpeeled" before extension
    let output_path = create_output_path(path);
    println!("\n=== Writing Output Image ===");
    println!("Output file: {}", output_path.display());
    
    // Write the image to the new file
    match write_png_image(&output_path, width, height, color_type, bit_depth, &trns, &buf) {
        Ok(_) => {
            println!("Successfully wrote image to: {}", output_path.display());
        }
        Err(e) => {
            eprintln!("Error writing output image: {}", e);
            std::process::exit(1);
        }
    }
}

fn add_randomized_noise(buf: &mut [u8], color_type: ColorType) {
    let mut rng = rand::thread_rng();
    
    match color_type {
        ColorType::Rgb => {
            // RGB: 3 bytes per pixel (R, G, B)
            for pixel in buf.chunks_exact_mut(3) {
                // Randomly select R (0), G (1), or B (2)
                let channel = rng.gen_range(0..3);
                // Randomly add or subtract 1
                let change: i16 = if rng.gen_bool(0.5) { 15 } else { -15 };
                let new_value = pixel[channel] as i16 + change;
                pixel[channel] = new_value.clamp(0, 255) as u8;
            }
        }
        ColorType::Rgba => {
            // RGBA: 4 bytes per pixel (R, G, B, A)
            for pixel in buf.chunks_exact_mut(4) {
                // Randomly select R (0), G (1), or B (2) - skip Alpha (3)
                let channel = rng.gen_range(0..3);
                // Randomly add or subtract 1
                let change: i16 = if rng.gen_bool(0.5) { 15 } else { -15 };
                let new_value = pixel[channel] as i16 + change;
                pixel[channel] = new_value.clamp(0, 255) as u8;
            }
        }
        ColorType::Grayscale => {
            // Grayscale: 1 byte per pixel
            for pixel in buf.iter_mut() {
                // Randomly add or subtract 1
                let change: i16 = if rng.gen_bool(0.5) { 15 } else { -15 };
                let new_value = *pixel as i16 + change;
                *pixel = new_value.clamp(0, 255) as u8;
            }
        }
        ColorType::GrayscaleAlpha => {
            // GrayscaleAlpha: 2 bytes per pixel (G, A)
            for pixel in buf.chunks_exact_mut(2) {
                // Modify the grayscale channel (0), skip Alpha (1)
                let change: i16 = if rng.gen_bool(0.5) { 15 } else { -15 };
                let new_value = pixel[0] as i16 + change;
                pixel[0] = new_value.clamp(0, 255) as u8;
            }
        }
        ColorType::Indexed => {
            // Indexed: 1 byte per pixel (palette index)
            // For indexed color, we modify the palette index value
            // This will change which color from the palette is used
            for pixel in buf.iter_mut() {
                // Randomly add or subtract 1 from the palette index
                let change: i16 = if rng.gen_bool(0.5) { 15 } else { -15 };
                let new_value = *pixel as i16 + change;
                *pixel = new_value.clamp(0, 255) as u8;
            }
        }
    }
}

fn create_output_path(input_path: &Path) -> PathBuf {
    let mut output_path = input_path.to_path_buf();
    
    // Get the file stem and extension
    if let Some(file_stem) = input_path.file_stem() {
        if let Some(extension) = input_path.extension() {
            // Create new filename with "-unpeeled" before extension
            let new_filename = format!("{}-unpeeled.{}", 
                file_stem.to_string_lossy(), 
                extension.to_string_lossy());
            output_path.set_file_name(new_filename);
        } else {
            // No extension, just append "-unpeeled"
            let new_filename = format!("{}-unpeeled", file_stem.to_string_lossy());
            output_path.set_file_name(new_filename);
        }
    } else {
        // Fallback: append "-unpeeled" to the path
        let mut path_str = input_path.to_string_lossy().to_string();
        path_str.push_str("-unpeeled");
        output_path = PathBuf::from(path_str);
    }
    
    output_path
}

fn write_png_image(
    output_path: &Path,
    width: u32,
    height: u32,
    color_type: png::ColorType,
    bit_depth: png::BitDepth,
    trns: &Option<Vec<u8>>,
    image_data: &[u8],
) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::create(output_path)?;
    let writer = BufWriter::new(file);
    
    let mut encoder = Encoder::new(writer, width, height);
    
    // Set only essential metadata: color type and bit depth
    encoder.set_color(color_type);
    encoder.set_depth(bit_depth);
    
    // Only include transparency (tRNS) if:
    // 1. The color type supports transparency via tRNS (Grayscale, RGB, or Indexed)
    // 2. AND transparency data actually exists
    // Note: GrayscaleAlpha and RgbAlpha have transparency built into pixel data, so tRNS is not needed
    match color_type {
        png::ColorType::Grayscale | png::ColorType::Rgb | png::ColorType::Indexed => {
            // These color types can use tRNS for transparency
            if let Some(trns_data) = trns {
                if !trns_data.is_empty() {
                    encoder.set_trns(trns_data.clone());
                }
            }
        }
        _ => {
            // GrayscaleAlpha and RgbAlpha have transparency in pixel data, tRNS not needed
        }
    }
    
    // Write header (creates IHDR chunk)
    let mut writer = encoder.write_header()?;
    // Write image data (creates IDAT chunks)
    writer.write_image_data(image_data)?;
    // Writer automatically closes with IEND chunk
    
    Ok(())
}
