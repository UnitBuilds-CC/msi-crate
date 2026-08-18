use cfb::CompoundFile;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

fn main() {
    let file = File::open("target/test_velocity_msi.msi").unwrap();
    let mut comp = CompoundFile::open(file).unwrap();
    
    let entries: Vec<(PathBuf, bool)> = comp.walk().map(|e| (e.path().to_path_buf(), e.is_stream())).collect();
    
    for (p, is_stream) in &entries {
        if !is_stream { continue; }
        let name = match p.file_name() {
            Some(n) => n.to_string_lossy().to_string(),
            None => continue,
        };
        
        let mut stream = comp.open_stream(p).unwrap();
        let mut data = Vec::new();
        stream.read_to_end(&mut data).unwrap();
        
        let cps: Vec<String> = name.chars().map(|c| format!("U+{:04X}", c as u32)).collect();
        println!("Stream '{}' ({} bytes) cps: {}", name, data.len(), cps.join(" "));
        
        // Show first 32 bytes as hex
        let hex: Vec<String> = data.iter().take(32).map(|b| format!("{:02X}", b)).collect();
        println!("  Data: {}", hex.join(" "));
        
        // Check if it starts with codepage (1252 = EC 04 00 00)
        if data.len() >= 4 {
            let first_u32 = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
            if first_u32 == 1252 {
                println!("  ** Starts with codepage 1252 - this is StringPool content!");
            }
        }
        println!();
    }
}
