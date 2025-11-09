use std::env;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read};
use std::path::{Path, PathBuf};
use png::{Decoder, Encoder};

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
        println!("Is file: {}", metadata.is_file());
        println!("Is directory: {}", metadata.is_dir());
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
    let interlaced = info.interlaced;
    let trns = info.trns.as_ref().map(|cow| cow.to_vec());
    let utf8_text = info.utf8_text.clone();
    
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
    
    // Get utf8_text from info
    for text in utf8_text {
        println!("Text: {}", text.get_text().unwrap());
    }
    
    // Basic image information
    println!("Width: {} pixels", width);
    println!("Height: {} pixels", height);
    println!("Color type: {:?}", color_type);
    println!("Bit depth: {:?}", bit_depth);
    println!("Bytes per pixel: {}", bytes_per_pixel);
    
    // Interlacing
    println!("Interlaced: {}", interlaced);
    
    // Additional info fields
    if let Some(ref trns_data) = trns {
        println!("Transparency: {:?}", trns_data);
    }
    
    // Read chunks for additional metadata
    println!("\n=== PNG Chunks (Metadata) ===");
    
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
    
    let mut chunks_found = false;
    
    // Read chunks: length (4 bytes), chunk type (4 bytes), data (length bytes), CRC (4 bytes)
    loop {
        let mut length_bytes = [0u8; 4];
        if file.read_exact(&mut length_bytes).is_err() {
            break;
        }
        let length = u32::from_be_bytes(length_bytes) as usize;
        
        let mut chunk_type_bytes = [0u8; 4];
        if file.read_exact(&mut chunk_type_bytes).is_err() {
            break;
        }
        let chunk_type = &chunk_type_bytes;
        
        let mut data = vec![0u8; length];
        if file.read_exact(&mut data).is_err() {
            break;
        }
        
        let mut crc_bytes = [0u8; 4];
        if file.read_exact(&mut crc_bytes).is_err() {
            break;
        }
        
        chunks_found = true;
        
        // Process chunks
        match chunk_type {
            b"tEXt" => {
                // tEXt chunk: keyword\0text
                if let Some(null_pos) = data.iter().position(|&b| b == 0) {
                    let keyword = String::from_utf8_lossy(&data[..null_pos]);
                    let text = String::from_utf8_lossy(&data[null_pos + 1..]);
                    println!("tEXt chunk - {}: {}", keyword, text);
                }
            }
            b"zTXt" => {
                // zTXt chunk: keyword\0compression_method\0compressed_text
                if let Some(null_pos) = data.iter().position(|&b| b == 0) {
                    let keyword = String::from_utf8_lossy(&data[..null_pos]);
                    if data.len() > null_pos + 1 {
                        let compression_method = data[null_pos + 1];
                        println!("zTXt chunk - {}: [compressed, method: {}]", keyword, compression_method);
                    }
                }
            }
            b"iTXt" => {
                // iTXt chunk: keyword\0compression_flag\0compression_method\0language_tag\0translated_keyword\0text
                if let Some(null_pos) = data.iter().position(|&b| b == 0) {
                    let keyword = String::from_utf8_lossy(&data[..null_pos]);
                    println!("iTXt chunk - {}: [international text]", keyword);
                }
            }
            b"pHYs" => {
                // pHYs chunk: 9 bytes - x_pixels_per_unit (4), y_pixels_per_unit (4), unit (1)
                if data.len() >= 9 {
                    let x = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
                    let y = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
                    let unit = data[8];
                    let unit_str = if unit == 1 { "meter" } else { "unknown" };
                    println!("pHYs chunk - X: {}, Y: {}, Unit: {}", x, y, unit_str);
                }
            }
            b"tIME" => {
                // tIME chunk: 7 bytes - year (2), month (1), day (1), hour (1), minute (1), second (1)
                if data.len() >= 7 {
                    let year = u16::from_be_bytes([data[0], data[1]]);
                    println!("tIME chunk - {}-{:02}-{:02} {:02}:{:02}:{:02}", 
                            year, data[2], data[3], data[4], data[5], data[6]);
                }
            }
            b"gAMA" => {
                // gAMA chunk: 4 bytes - gamma value
                if data.len() >= 4 {
                    let gamma = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
                    println!("gAMA chunk - Gamma: {}", gamma as f64 / 100000.0);
                }
            }
            b"cHRM" => {
                // cHRM chunk: 32 bytes - white point (8), red (8), green (8), blue (8)
                if data.len() >= 32 {
                    println!("cHRM chunk - [chromaticity data present]");
                }
            }
            b"sRGB" => {
                // sRGB chunk: 1 byte - rendering intent
                if !data.is_empty() {
                    let intent = match data[0] {
                        0 => "Perceptual",
                        1 => "Relative colorimetric",
                        2 => "Saturation",
                        3 => "Absolute colorimetric",
                        _ => "Unknown",
                    };
                    println!("sRGB chunk - Rendering intent: {}", intent);
                }
            }
            b"iCCP" => {
                // iCCP chunk: profile_name\0compression_method\0compressed_profile
                if let Some(null_pos) = data.iter().position(|&b| b == 0) {
                    let profile_name = String::from_utf8_lossy(&data[..null_pos]);
                    if data.len() > null_pos + 1 {
                        let compression_method = data[null_pos + 1];
                        let profile_size = data.len() - null_pos - 2;
                        println!("iCCP chunk - Profile name: {}", profile_name);
                        println!("  Compression method: {}", compression_method);
                        println!("  Profile size: {} bytes", profile_size);
                    }
                }
            }
            b"IEND" => {
                // IEND chunk marks the end
                break;
            }
            _ => {
                // Other chunk types
                let chunk_name = String::from_utf8_lossy(chunk_type);
                println!("Other chunk - {}: {} bytes", chunk_name, data.len());
            }
        }
    }
    
    if !chunks_found {
        println!("No additional metadata chunks found in PNG file.");
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
    
    // Set color type and bit depth
    encoder.set_color(color_type);
    encoder.set_depth(bit_depth);
    
    // Copy other info fields
    if let Some(trns_data) = trns {
        encoder.set_trns(trns_data.clone());
    }
    
    let mut writer = encoder.write_header()?;
    writer.write_image_data(image_data)?;
    
    Ok(())
}
