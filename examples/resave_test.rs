use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;

fn main() {
    // Read our MSI
    let mut file = File::open("test_simple.msi").unwrap();
    let mut data = Vec::new();
    file.read_to_end(&mut data).unwrap();
    
    println!("Original MSI: {} bytes", data.len());
    
    // Open with cfb
    let cursor = std::io::Cursor::new(&data);
    let mut comp = cfb::CompoundFile::open(cursor).unwrap();
    
    // Collect all streams
    let entries: Vec<(PathBuf, bool)> = comp.walk()
        .map(|e| (e.path().to_path_buf(), e.is_stream()))
        .collect();
    
    let mut streams = Vec::new();
    for (path, is_stream) in &entries {
        if *is_stream {
            let name = path.file_name().unwrap().to_string_lossy().to_string();
            let mut stream = comp.open_stream(path).unwrap();
            let mut stream_data = Vec::new();
            stream.read_to_end(&mut stream_data).unwrap();
            streams.push((path.clone(), name, stream_data));
        }
    }
    
    println!("Found {} streams", streams.len());
    
    // Create a new CFB file with the same streams
    let output_cursor = std::io::Cursor::new(Vec::new());
    let mut out = cfb::OpenOptions::new()
        .create_with(output_cursor)
        .unwrap();
    
    for (path, name, stream_data) in &streams {
        println!("  Adding: {} ({} bytes)", name, stream_data.len());
        out.create_storage(path.parent().unwrap().to_string_lossy().as_ref()).ok();
        let mut stream = out.create_stream(path.file_name().unwrap().to_string_lossy().as_ref()).unwrap();
        stream.write_all(stream_data).unwrap();
    }
    
    let output_data = out.into_inner().into_inner();
    std::fs::write("test_resaved.msi", &output_data).unwrap();
    
    println!("Resaved MSI: {} bytes", output_data.len());
}
