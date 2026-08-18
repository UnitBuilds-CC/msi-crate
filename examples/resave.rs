use cfb::CompoundFile;
use std::fs::File;
use std::io::{Cursor, Read, Write};
use std::path::PathBuf;

fn main() {
    // Read our MSI
    let file = File::open("target/test_velocity_msi.msi").unwrap();
    let mut comp = CompoundFile::open(file).unwrap();
    
    // Collect all streams
    let entries: Vec<(PathBuf, bool)> = comp.walk().map(|e| (e.path().to_path_buf(), e.is_stream())).collect();
    
    let mut streams: Vec<(PathBuf, Vec<u8>)> = Vec::new();
    for (p, is_stream) in &entries {
        if *is_stream {
            let mut stream = comp.open_stream(p).unwrap();
            let mut data = Vec::new();
            stream.read_to_end(&mut data).unwrap();
            streams.push((p.clone(), data));
        }
    }
    
    // Get root CLSID (use known MSI CLSID)
    let clsid = uuid::Uuid::parse_str("000c1084-0000-0000-c000-000000000046").unwrap();
    println!("Using MSI CLSID: {}", clsid);
    
    // Create a new V4 OLE file and write everything back
    let cursor = Cursor::new(Vec::new());
    let mut new_comp = CompoundFile::create_with_version(cfb::Version::V4, cursor).unwrap();
    new_comp.set_storage_clsid("/", clsid).unwrap();
    
    for (path, data) in &streams {
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        println!("Writing stream: '{}' ({} bytes)", name, data.len());
        let mut stream = new_comp.create_stream(name.as_str()).unwrap();
        stream.write_all(data).unwrap();
        drop(stream);
    }
    
    let cursor = new_comp.into_inner();
    std::fs::write("target/resaved_v4.msi", cursor.into_inner()).unwrap();
    println!("\nCreated target/resaved_v4.msi");
    
    // Also try V3
    let cursor = Cursor::new(Vec::new());
    let mut new_comp = CompoundFile::create_with_version(cfb::Version::V3, cursor).unwrap();
    new_comp.set_storage_clsid("/", clsid).unwrap();
    
    for (path, data) in &streams {
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let mut stream = new_comp.create_stream(name.as_str()).unwrap();
        stream.write_all(data).unwrap();
        drop(stream);
    }
    
    let cursor = new_comp.into_inner();
    std::fs::write("target/resaved_v3.msi", cursor.into_inner()).unwrap();
    println!("Created target/resaved_v3.msi");
}
