use velocity_msi::*;
use std::io::Read;
use std::path::PathBuf;

fn main() {
    // Create multi-table MSI and dump all stream info
    let mut b = MsiBuilder::new();
    b.set_title("Test MSI multi-table");
    b.set_template("x64", 1033);

    b.create_table("Property", vec![
        Column::build("Property").string(64).primary_key().build(),
        Column::build("Value").string(255).nullable().build(),
    ]).unwrap();
    b.insert_rows("Property", vec![
        vec![Value::from("ProductName"), Value::from("Test")],
    ]).unwrap();

    b.create_table("Directory", vec![
        Column::build("Directory").string(64).primary_key().build(),
        Column::build("Directory_Parent").string(64).nullable().build(),
        Column::build("DefaultDir").string(255).nullable().build(),
    ]).unwrap();
    b.insert_rows("Directory", vec![
        vec![Value::from("TARGETDIR"), Value::Null, Value::from(".")],
    ]).unwrap();

    let data = b.build().unwrap();
    std::fs::write("test_debug.msi", &data).unwrap();
    
    println!("=== Multi-table MSI: {} bytes ===", data.len());
    
    // Read back with cfb
    let file = std::fs::File::open("test_debug.msi").unwrap();
    match cfb::CompoundFile::open(file) {
        Ok(mut compound) => {
            println!("\n=== OLE Streams ===");
            
            // Collect paths first to avoid borrow issues
            let entries: Vec<(PathBuf, bool)> = compound.walk()
                .map(|e| (e.path().to_path_buf(), e.is_stream()))
                .collect();
            
            for (path, is_stream) in entries {
                if !is_stream { continue; }
                let name = path.file_name().unwrap().to_string_lossy().to_string();
                let mut stream = compound.open_stream(&path).unwrap();
                let mut stream_data = Vec::new();
                stream.read_to_end(&mut stream_data).unwrap();
                
                println!("\n--- {} ({} bytes) ---", name, stream_data.len());
                
                // Decode stream name to see what it represents
                let cps: Vec<u16> = name.encode_utf16().collect();
                if !cps.is_empty() && cps[0] == 0x4840 {
                    println!("  [TABLE PREFIX]");
                }
                
                print_hex(&stream_data, 256);
            }
        }
        Err(e) => println!("Failed to open as CFB: {}", e),
    }
}

fn print_hex(data: &[u8], max_bytes: usize) {
    let len = data.len().min(max_bytes);
    for i in (0..len).step_by(16) {
        print!("{:04x}: ", i);
        for j in 0..16 {
            if i + j < len {
                print!("{:02x} ", data[i + j]);
            } else {
                print!("   ");
            }
        }
        print!(" ");
        for j in 0..16 {
            if i + j < len {
                let c = data[i + j];
                if c >= 0x20 && c < 0x7f {
                    print!("{}", c as char);
                } else {
                    print!(".");
                }
            }
        }
        println!();
    }
    if data.len() > max_bytes {
        println!("... ({} more bytes)", data.len() - max_bytes);
    }
}
