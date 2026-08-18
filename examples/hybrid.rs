use cfb::CompoundFile;
use std::fs::File;
use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};

fn main() {
    // Read system MSI (known working)
    let sys_file = File::open("C:\\Windows\\Installer\\10d16cbb.msi").unwrap();
    let mut sys_comp = CompoundFile::open(sys_file).unwrap();
    
    // Read our MSI
    let our_file = File::open("target/test_velocity_msi.msi").unwrap();
    let mut our_comp = CompoundFile::open(our_file).unwrap();
    
    // Collect our stream data (keyed by codepoints)
    let our_entries: Vec<(PathBuf, bool)> = our_comp.walk().map(|e| (e.path().to_path_buf(), e.is_stream())).collect();
    let mut our_streams: Vec<(String, Vec<u8>)> = Vec::new();
    for (p, is_stream) in &our_entries {
        if !is_stream { continue; }
        let name = p.file_name().unwrap().to_string_lossy().to_string();
        let mut stream = our_comp.open_stream(p).unwrap();
        let mut data = Vec::new();
        stream.read_to_end(&mut data).unwrap();
        our_streams.push((name, data));
    }
    
    // Identify our table streams by their codepoints
    // _Tables:  U+4840 U+3F7F U+4164 U+422F U+4836
    // _Columns: U+4840 U+3B3F U+43F2 U+4438 U+45B1
    // _Validation: U+4840 U+3FFF U+43E4 U+41EC U+45E4 U+44AC U+4831
    // _StringPool: U+3F3F U+4577 U+446C U+3E6A U+44B2 U+482F
    // _StringData: U+3F3F U+4577 U+446C U+3B6A U+45E4 U+4824
    
    let our_tables_name = "\u{4840}\u{3F7F}\u{4164}\u{422F}\u{4836}";
    let our_columns_name = "\u{4840}\u{3B3F}\u{43F2}\u{4438}\u{45B1}";
    let our_validation_name = "\u{4840}\u{3FFF}\u{43E4}\u{41EC}\u{45E4}\u{44AC}\u{4831}";
    let our_stringpool_name = "\u{3F3F}\u{4577}\u{446C}\u{3E6A}\u{44B2}\u{482F}";
    let our_stringdata_name = "\u{3F3F}\u{4577}\u{446C}\u{3B6A}\u{45E4}\u{4824}";
    
    let our_names = vec![
        our_tables_name, our_columns_name, our_validation_name,
        our_stringpool_name, our_stringdata_name,
    ];
    
    // Find our data for each
    let mut our_data: Vec<(&str, &[u8])> = Vec::new();
    for name in &our_names {
        for (n, d) in &our_streams {
            if n == name {
                our_data.push((name, d.as_slice()));
                break;
            }
        }
    }
    
    // Create new file: system MSI streams but replace table streams with ours
    let sys_entries: Vec<(PathBuf, bool)> = sys_comp.walk().map(|e| (e.path().to_path_buf(), e.is_stream())).collect();
    
    let cursor = Cursor::new(Vec::new());
    let mut new_comp = CompoundFile::create_with_version(cfb::Version::V4, cursor).unwrap();
    let clsid = uuid::Uuid::parse_str("000c1084-0000-0000-c000-000000000046").unwrap();
    new_comp.set_storage_clsid("/", clsid).unwrap();
    
    for (p, is_stream) in &sys_entries {
        if !is_stream { continue; }
        let name = p.file_name().unwrap().to_string_lossy().to_string();
        
        // Check if this is a table stream we should replace
        let is_our_table = our_names.contains(&name.as_str());
        
        if is_our_table {
            // Use our version
            for (n, d) in &our_data {
                if *n == name.as_str() {
                    println!("Replacing '{}' with our data ({} bytes)", name, d.len());
                    let mut s = new_comp.create_stream(p.as_path()).unwrap();
                    s.write_all(d).unwrap();
                    drop(s);
                    break;
                }
            }
        } else {
            // Use system version
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
    std::fs::write("target/hybrid.msi", &data).unwrap();
    println!("Created target/hybrid.msi ({} bytes)", data.len());
}
