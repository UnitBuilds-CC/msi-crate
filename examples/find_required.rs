use cfb::CompoundFile;
use std::fs::File;
use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};

fn create_with_streams(stream_names: &[&str]) -> String {
    let sys_file = File::open("C:\\Windows\\Installer\\10d16cbb.msi").unwrap();
    let mut sys_comp = CompoundFile::open(sys_file).unwrap();
    
    let cursor = Cursor::new(Vec::new());
    let mut new_comp = CompoundFile::create_with_version(cfb::Version::V4, cursor).unwrap();
    let clsid = uuid::Uuid::parse_str("000c1084-0000-0000-c000-000000000046").unwrap();
    new_comp.set_storage_clsid("/", clsid).unwrap();
    
    // Always include SummaryInformation
    let si_path = Path::new("\u{0005}SummaryInformation");
    let mut si_stream = sys_comp.open_stream(si_path).unwrap();
    let mut si_data = Vec::new();
    si_stream.read_to_end(&mut si_data).unwrap();
    let mut s = new_comp.create_stream(si_path).unwrap();
    s.write_all(&si_data).unwrap();
    drop(s);
    
    // Add requested streams
    let entries: Vec<(PathBuf, bool)> = sys_comp.walk().map(|e| (e.path().to_path_buf(), e.is_stream())).collect();
    for (p, is_stream) in &entries {
        if !is_stream { continue; }
        let name = p.file_name().unwrap().to_string_lossy().to_string();
        if stream_names.contains(&name.as_str()) {
            let mut stream = sys_comp.open_stream(p).unwrap();
            let mut data = Vec::new();
            stream.read_to_end(&mut data).unwrap();
            let mut s = new_comp.create_stream(p.as_path()).unwrap();
            s.write_all(&data).unwrap();
            drop(s);
        }
    }
    
    let cursor = new_comp.into_inner();
    let data = cursor.into_inner();
    let filename = format!("target/test_{}.msi", stream_names.join("_"));
    std::fs::write(&filename, &data).unwrap();
    filename
}

fn main() {
    // Decode system MSI stream names to find the encoded names
    let sys_file = File::open("C:\\Windows\\Installer\\10d16cbb.msi").unwrap();
    let mut sys_comp = CompoundFile::open(sys_file).unwrap();
    
    let entries: Vec<(PathBuf, bool)> = sys_comp.walk().map(|e| (e.path().to_path_buf(), e.is_stream())).collect();
    println!("System MSI streams:");
    for (p, is_stream) in &entries {
        if !is_stream { continue; }
        let name = p.file_name().unwrap().to_string_lossy().to_string();
        let cps: Vec<String> = name.chars().map(|c| format!("U+{:04X}", c as u32)).collect();
        println!("  '{}' cps: {}", name, cps.join(" "));
    }
}
