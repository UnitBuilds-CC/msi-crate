/// List streams in good_cab_test.msi
fn main() {
    use std::io::Cursor;
    let data = std::fs::read("C:\\temp\\good_cab_test.msi").unwrap();
    let cursor = Cursor::new(&data);
    let comp = cfb::CompoundFile::open(cursor).unwrap();
    
    println!("Streams in good_cab_test.msi:");
    for entry in comp.walk() {
        if entry.is_stream() {
            let name = entry.name();
            println!("  '{}'", name);
            // Show UTF-16LE bytes
            let bytes: Vec<u8> = name.encode_utf16()
                .flat_map(|c| c.to_le_bytes().to_vec())
                .collect();
            print!("    ");
            for b in &bytes { print!("{:02x} ", b); }
            println!();
        }
    }
}
