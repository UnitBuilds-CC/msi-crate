use velocity_msi::*;
use std::io::Read;
use std::path::PathBuf;

fn main() {
    // Create ultra-simple MSI
    let mut b = MsiBuilder::new();
    b.set_title("Ultra simple");

    b.create_table("Alpha", vec![
        Column::build("Key").string(64).primary_key().build(),
    ]).unwrap();
    b.insert_rows("Alpha", vec![
        vec![Value::from("A1")],
    ]).unwrap();

    b.create_table("Beta", vec![
        Column::build("Key").string(64).primary_key().build(),
    ]).unwrap();
    b.insert_rows("Beta", vec![
        vec![Value::from("B1")],
    ]).unwrap();

    let data = b.build().unwrap();
    std::fs::write("test_ultra.msi", &data).unwrap();
    
    println!("=== Ultra-simple MSI: {} bytes ===", data.len());
    println!("\nString pool should be:");
    println!("  1: A1");
    println!("  2: B1");
    println!("  3: Alpha");
    println!("  4: Beta");
    println!("  5: Key");
    println!("  6: N");
    
    // Read back with cfb
    let file = std::fs::File::open("test_ultra.msi").unwrap();
    let mut compound = cfb::CompoundFile::open(file).unwrap();
    
    let entries: Vec<(PathBuf, bool)> = compound.walk()
        .map(|e| (e.path().to_path_buf(), e.is_stream()))
        .collect();
    
    println!("\n=== Streams ===");
    for (path, is_stream) in entries {
        if !is_stream { continue; }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let mut stream = compound.open_stream(&path).unwrap();
        let mut stream_data = Vec::new();
        stream.read_to_end(&mut stream_data).unwrap();
        
        let cps: Vec<u16> = name.encode_utf16().collect();
        let prefix = if !cps.is_empty() && cps[0] == 0x4840 { "T" } else { "N" };
        println!("\n[{}] {} ({} bytes)", prefix, name, stream_data.len());
        
        // Decode known stream names
        if name == "\u{4840}\u{3f7f}\u{4164}\u{422f}\u{4836}" {
            println!("  → _Tables");
            print!("_Tables data: ");
            for b in &stream_data { print!("{:02x} ", b); }
            println!();
        } else if name == "\u{4840}\u{3b3f}\u{43f2}\u{4438}\u{45b1}" {
            println!("  → _Columns");
            print!("_Columns data: ");
            for b in &stream_data { print!("{:02x} ", b); }
            println!();
        } else if name.contains('\u{4840}') && stream_data.len() == 2 {
            println!("  → User table (2 bytes = 1 row × 1 string col)");
            let id = u16::from_le_bytes([stream_data[0], stream_data[1]]);
            println!("  String pool ID: {}", id);
        }
    }
}
