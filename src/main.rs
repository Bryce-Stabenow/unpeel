use std::env;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;
use png::Decoder;

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
    
    let reader = match decoder.read_info() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Error reading PNG: {}", e);
            std::process::exit(1);
        }
    };
    
    let info = reader.info();
    
    // Get utf8_text from info
    let utf8_text = info.utf8_text.clone();
    for text in utf8_text {
        println!("Text: {}", text.get_text().unwrap());
    }
    
    // Basic image information
    println!("Width: {} pixels", info.width);
    println!("Height: {} pixels", info.height);
    println!("Color type: {:?}", info.color_type);
    println!("Bit depth: {:?}", info.bit_depth);
    println!("Bytes per pixel: {}", info.bytes_per_pixel());
    
    // Interlacing
    println!("Interlaced: {}", info.interlaced);
    
    // Additional info fields
    if let Some(trns) = &info.trns {
        println!("Transparency: {:?}", trns);
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
    println!("Dimensions: {}x{}", info.width, info.height);
    println!("Color format: {:?} at {:?} bits", info.color_type, info.bit_depth);
}
