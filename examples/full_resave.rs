use cfb::CompoundFile;
use std::fs::File;
use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};

fn main() {
    // Read system MSI
    let sys_file = File::open("C:\\Windows\\Installer\\10d16cbb.msi").unwrap();
    let mut sys_comp = CompoundFile::open(sys_file).unwrap();
    
    // Collect all system streams
    let entries: Vec<(PathBuf, bool)> = sys_comp.walk().map(|e| (e.path().to_path_buf(), e.is_stream())).collect();
    let mut streams: Vec<(PathBuf, Vec<u8>)> = Vec::new();
    for (p, is_stream) in &entries {
        if *is_stream {
            let mut stream = sys_comp.open_stream(p).unwrap();
            let mut data = Vec::new();
            stream.read_to_end(&mut data).unwrap();
            streams.push((p.clone(), data));
        }
    }
    println!("Collected {} system streams", streams.len());
    
    // Create new V4 file with MSI CLSID and ALL system streams
    let cursor = Cursor::new(Vec::new());
    let mut new_comp = CompoundFile::create_with_version(cfb::Version::V4, cursor).unwrap();
    let clsid = uuid::Uuid::parse_str("000c1084-0000-0000-c000-000000000046").unwrap();
    new_comp.set_storage_clsid("/", clsid).unwrap();
    
    for (path, data) in &streams {
        let mut stream = new_comp.create_stream(path.as_path()).unwrap();
        stream.write_all(data).unwrap();
        drop(stream);
    }
    
    let cursor = new_comp.into_inner();
    let data = cursor.into_inner();
    std::fs::write("target/full_resaved.msi", &data).unwrap();
    println!("Created target/full_resaved.msi ({} bytes)", data.len());
}
