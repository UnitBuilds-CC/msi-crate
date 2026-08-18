use cfb::CompoundFile;
use std::fs::File;
use std::io::{Cursor, Read, Write};
use std::path::PathBuf;

fn main() {
    // Read a known-good system MSI
    let system_msi = "C:\\Windows\\Installer\\10d16cbb.msi";
    println!("Reading system MSI: {}", system_msi);
    let file = File::open(system_msi).unwrap();
    let mut comp = CompoundFile::open(file).unwrap();
    
    // Collect all streams
    let entries: Vec<(PathBuf, bool)> = comp.walk().map(|e| (e.path().to_path_buf(), e.is_stream())).collect();
    println!("Found {} entries", entries.len());
    
    let mut streams: Vec<(PathBuf, Vec<u8>)> = Vec::new();
    for (p, is_stream) in &entries {
        if *is_stream {
            let mut stream = comp.open_stream(p).unwrap();
            let mut data = Vec::new();
            stream.read_to_end(&mut data).unwrap();
            streams.push((p.clone(), data));
        }
    }
    println!("Collected {} streams", streams.len());
    
    // Resave as V4
    let cursor = Cursor::new(Vec::new());
    let mut new_comp = CompoundFile::create_with_version(cfb::Version::V4, cursor).unwrap();
    
    // CRITICAL: Set the MSI CLSID on root storage
    let clsid = uuid::Uuid::parse_str("000c1084-0000-0000-c000-000000000046").unwrap();
    new_comp.set_storage_clsid("/", clsid).unwrap();
    
    for (path, data) in &streams {
        let mut stream = new_comp.create_stream(path.as_path()).unwrap();
        stream.write_all(data).unwrap();
        drop(stream);
    }
    
    let cursor = new_comp.into_inner();
    let resaved_data = cursor.into_inner();
    std::fs::write("target/resaved_system.msi", &resaved_data).unwrap();
    println!("Created resaved_system.msi: {} bytes", resaved_data.len());
    
    // Compare sizes
    let original_size = std::fs::metadata(system_msi).unwrap().len();
    println!("Original: {} bytes, Resaved: {} bytes", original_size, resaved_data.len());
}
