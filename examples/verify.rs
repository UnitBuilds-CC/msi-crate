use cfb::CompoundFile;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

fn main() {
    let path = "target/test_velocity.msi";
    
    // First check the raw header bytes
    let raw = std::fs::read(path).unwrap();
    println!("File size: {} bytes", raw.len());
    println!("Header magic: {:02X?}", &raw[0..8]);
    println!("Major version: {}", u16::from_le_bytes([raw[26], raw[27]]));
    println!("Sector shift: {}", u16::from_le_bytes([raw[28], raw[29]]));
    println!("Mini sector shift: {}", u16::from_le_bytes([raw[30], raw[31]]));
    println!("First dir sector: {}", u32::from_le_bytes([raw[48], raw[49], raw[50], raw[51]]));
    println!("First mini FAT sector: {}", u32::from_le_bytes([raw[60], raw[61], raw[62], raw[63]]));
    println!("DIFAT[0]: {}", u32::from_le_bytes([raw[76], raw[77], raw[78], raw[79]]));
    
    // Check CLSID in root directory entry (sector 0 = offset 4096, entry 0, offset 80)
    let dir_off = 4096;
    let clsid_off = dir_off + 80;
    print!("Root CLSID: ");
    for i in 0..16 {
        print!("{:02X}", raw[clsid_off + i]);
    }
    println!();
    
    // Check root entry type
    println!("Root entry type: {} (should be 5)", raw[dir_off + 66]);
    println!("Root start sector: {}", u32::from_le_bytes([
        raw[dir_off + 116], raw[dir_off + 117], raw[dir_off + 118], raw[dir_off + 119]
    ]));
    println!("Root stream size: {}", u64::from_le_bytes([
        raw[dir_off + 120], raw[dir_off + 121], raw[dir_off + 122], raw[dir_off + 123],
        raw[dir_off + 124], raw[dir_off + 125], raw[dir_off + 126], raw[dir_off + 127]
    ]));
    println!("Root child DID: {}", i32::from_le_bytes([
        raw[dir_off + 76], raw[dir_off + 77], raw[dir_off + 78], raw[dir_off + 79]
    ]));
    
    // Print all directory entries
    println!("\nDirectory entries:");
    for did in 0..32 {
        let off = dir_off + did * 128;
        if off + 128 > raw.len() { break; }
        
        let obj_type = raw[off + 66];
        if obj_type == 0 { continue; } // empty
        
        let name_len = u16::from_le_bytes([raw[off + 64], raw[off + 65]]) as usize;
        let name_chars: Vec<u16> = (0..(name_len / 2 - 1))
            .map(|i| u16::from_le_bytes([raw[off + i * 2], raw[off + i * 2 + 1]]))
            .collect();
        let name = String::from_utf16_lossy(&name_chars);
        
        let left = i32::from_le_bytes([raw[off + 68], raw[off + 69], raw[off + 70], raw[off + 71]]);
        let right = i32::from_le_bytes([raw[off + 72], raw[off + 73], raw[off + 74], raw[off + 75]]);
        let child = i32::from_le_bytes([raw[off + 76], raw[off + 77], raw[off + 78], raw[off + 79]]);
        let start = u32::from_le_bytes([raw[off + 116], raw[off + 117], raw[off + 118], raw[off + 119]]);
        let size = u64::from_le_bytes([
            raw[off + 120], raw[off + 121], raw[off + 122], raw[off + 123],
            raw[off + 124], raw[off + 125], raw[off + 126], raw[off + 127]
        ]);
        
        let clsid_bytes = &raw[off + 80..off + 96];
        let has_clsid = clsid_bytes.iter().any(|&b| b != 0);
        
        println!("  DID {}: type={} name='{}' L={} R={} child={} start={} size={} clsid={}",
            did, obj_type, name, left, right, child, start, size,
            if has_clsid { "YES" } else { "no" });
        
        // Print name codepoints
        let cps: Vec<String> = name.chars().map(|c| format!("U+{:04X}", c as u32)).collect();
        println!("    codepoints: {}", cps.join(" "));
    }
    
    // Now try reading with cfb
    println!("\nTrying to read with cfb crate...");
    let file = File::open(path).unwrap();
    match CompoundFile::open(file) {
        Ok(mut comp) => {
            println!("cfb opened file successfully!");
            
            let entries: Vec<(PathBuf, bool)> = comp.walk()
                .map(|e| (e.path().to_path_buf(), e.is_stream()))
                .collect();
            
            println!("Found {} entries:", entries.len());
            for (p, is_stream) in &entries {
                if *is_stream {
                    let name = p.file_name().unwrap().to_string_lossy().to_string();
                    let mut stream = comp.open_stream(p).unwrap();
                    let mut data = Vec::new();
                    stream.read_to_end(&mut data).unwrap();
                    println!("  Stream '{}' ({} bytes)", name, data.len());
                    let cps: Vec<String> = name.chars().map(|c| format!("U+{:04X}", c as u32)).collect();
                    println!("    codepoints: {}", cps.join(" "));
                } else {
                    println!("  Storage '{}'", p.display());
                }
            }
        }
        Err(e) => {
            println!("cfb FAILED to open: {}", e);
        }
    }
}
