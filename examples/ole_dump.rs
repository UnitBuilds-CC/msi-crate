use std::io::{Cursor, Read};

fn main() {
    let vel_data = std::fs::read("our_msi.msi").unwrap();
    let ref_data = std::fs::read("ref_msi.msi").unwrap();

    println!("=== Velocity MSI OLE structure ===");
    dump_ole_streams(&vel_data);

    println!("\n=== Reference MSI OLE structure ===");
    dump_ole_streams(&ref_data);
}

fn dump_ole_streams(data: &[u8]) {
    let cursor = Cursor::new(data);
    let mut comp = cfb::CompoundFile::open(cursor).unwrap();

    let entries: Vec<_> = comp.walk()
        .map(|e| (e.path().to_string_lossy().to_string(), e.is_stream(), e.is_storage()))
        .collect();

    for (path, is_stream, is_storage) in &entries {
        if *is_stream {
            let mut stream = comp.open_stream(path).unwrap();
            let mut buf = Vec::new();
            stream.read_to_end(&mut buf).unwrap();
            println!("  Stream: {:?} ({} bytes)", path, buf.len());
            // Print first 32 bytes hex
            print!("    ");
            for (i, b) in buf[..32.min(buf.len())].iter().enumerate() {
                print!("{:02X} ", b);
                if i % 16 == 15 { print!("\n    "); }
            }
            println!();
        } else if *is_storage {
            println!("  Storage: {:?}", path);
        }
    }
}
