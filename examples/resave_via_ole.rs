use cfb::CompoundFile;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

fn main() {
    // Read system MSI (known working)
    let sys_file = File::open("C:\\Windows\\Installer\\10d16cbb.msi").unwrap();
    let mut sys_comp = CompoundFile::open(sys_file).unwrap();
    
    // Collect ALL system streams
    let entries: Vec<(PathBuf, bool)> = sys_comp.walk()
        .map(|e| (e.path().to_path_buf(), e.is_stream()))
        .collect();
    
    let mut streams = Vec::new();
    for (p, is_stream) in &entries {
        if !is_stream { continue; }
        let name = p.file_name().unwrap().to_string_lossy().to_string();
        let mut stream = sys_comp.open_stream(p).unwrap();
        let mut data = Vec::new();
        stream.read_to_end(&mut data).unwrap();
        println!("Read stream '{}' ({} bytes)", name, data.len());
        streams.push(velocity_msi::ole::OleStream { name, data });
    }
    
    // Build OLE file with our writer
    let ole_data = velocity_msi::ole::build_ole_file(&streams);
    std::fs::write("target/resaved_system.msi", &ole_data).unwrap();
    println!("\nCreated target/resaved_system.msi ({} bytes)", ole_data.len());
}
