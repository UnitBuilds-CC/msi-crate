/// Try to open velocity_comp.msi with the msi crate to get a specific error.
use std::io::Cursor;

fn main() {
    let ws_root = env!("CARGO_MANIFEST_DIR").to_string() + "/../..";
    
    // Try opening our MSI
    let our_path = format!("{}/velocity_comp.msi", ws_root);
    println!("=== Opening our MSI with msi crate ===");
    let our_data = std::fs::read(&our_path).unwrap();
    println!("File size: {} bytes", our_data.len());
    
    match msi::Package::open(Cursor::new(&our_data)) {
        Ok(pkg) => {
            println!("SUCCESS: Opened our MSI");
            println!("Tables:");
            for table in pkg.tables() {
                println!("  {:?}", table.name());
            }
        }
        Err(e) => {
            println!("ERROR opening our MSI: {:?}", e);
        }
    }
    
    // Try opening reference MSI
    let ref_path = format!("{}/python_ref.msi", ws_root);
    println!("\n=== Opening reference MSI with msi crate ===");
    let ref_data = std::fs::read(&ref_path).unwrap();
    println!("File size: {} bytes", ref_data.len());
    
    match msi::Package::open(Cursor::new(&ref_data)) {
        Ok(pkg) => {
            println!("SUCCESS: Opened reference MSI");
            println!("Tables (first 10):");
            for (i, table) in pkg.tables().enumerate() {
                if i < 10 { println!("  {:?}", table.name()); }
            }
            let total = pkg.tables().count();
            if total > 10 { println!("  ... and {} more", total - 10); }
            println!("Total tables: {}", total);
        }
        Err(e) => {
            println!("ERROR opening reference MSI: {:?}", e);
        }
    }
    
    // Now try to read specific streams from our MSI
    println!("\n=== Reading our MSI streams via cfb ===");
    let mut comp = cfb::CompoundFile::open(Cursor::new(&our_data)).unwrap();
    let paths: Vec<_> = comp.walk()
        .filter(|e| e.is_stream())
        .map(|e| (e.path().to_path_buf(), e.is_stream()))
        .collect();
    for (path, _) in &paths {
        let name = path.to_string_lossy();
        let mut stream = comp.open_stream(path).unwrap();
        let mut data = Vec::new();
        std::io::Read::read_to_end(&mut stream, &mut data).unwrap();
        println!("  {} ({} bytes)", name, data.len());
        
        // For _StringPool, decode the header
        if name.contains("\u{4840}\u{3b3f}") { // _StringPool encoded
            if data.len() >= 4 {
                let header = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
                let codepage = header & 0xFFFF;
                let long_refs = (header >> 31) != 0;
                println!("    StringPool header: codepage={}, long_refs={}", codepage, long_refs);
                let num_entries = (data.len() - 4) / 4;
                println!("    Entries: {}", num_entries);
            }
        }
    }
}
