use cfb::CompoundFile;
use std::fs::File;
use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};

fn main() {
    // Read our MSI
    let file = File::open("target/test_velocity_msi.msi").unwrap();
    let mut our_comp = CompoundFile::open(file).unwrap();
    
    // Read the system MSI's SummaryInformation
    let sys_file = File::open("C:\\Windows\\Installer\\10d16cbb.msi").unwrap();
    let mut sys_comp = CompoundFile::open(sys_file).unwrap();
    
    let si_path = Path::new("\u{0005}SummaryInformation");
    let mut sys_si = sys_comp.open_stream(si_path).unwrap();
    let mut sys_si_data = Vec::new();
    sys_si.read_to_end(&mut sys_si_data).unwrap();
    println!("System SummaryInformation: {} bytes", sys_si_data.len());
    
    // Collect all our streams
    let entries: Vec<(PathBuf, bool)> = our_comp.walk().map(|e| (e.path().to_path_buf(), e.is_stream())).collect();
    let mut streams: Vec<(PathBuf, Vec<u8>)> = Vec::new();
    for (p, is_stream) in &entries {
        if *is_stream {
            let name = p.file_name().unwrap().to_string_lossy().to_string();
            if name == "\u{0005}SummaryInformation" {
                // Replace with system's SummaryInformation
                println!("Replacing our SummaryInformation with system's");
                streams.push((p.clone(), sys_si_data.clone()));
            } else {
                let mut stream = our_comp.open_stream(p).unwrap();
                let mut data = Vec::new();
                stream.read_to_end(&mut data).unwrap();
                streams.push((p.clone(), data));
            }
        }
    }
    
    // Create new V4 file
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
    std::fs::write("target/test_with_sys_si.msi", cursor.into_inner()).unwrap();
    println!("Created target/test_with_sys_si.msi");
}
