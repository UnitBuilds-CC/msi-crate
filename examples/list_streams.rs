/// List all OLE streams in an MSI file
/// cargo run --example list_streams -p velocity-msi
use std::io::Cursor;

fn main() {
    let path = "C:\\temp\\complete_test.msi";
    let data = std::fs::read(path).unwrap();
    println!("MSI: {} bytes", data.len());

    let cursor = Cursor::new(&data);
    let comp = cfb::CompoundFile::open(cursor).unwrap();

    println!("\nStreams in MSI:");
    let entries: Vec<_> = comp.walk().collect();
    for entry in &entries {
        if entry.is_stream() {
            let name = entry.name();
            println!("  Stream: '{}'", name);
            // Show raw bytes of the name
            let name_bytes: Vec<u8> = name.encode_utf16()
                .flat_map(|c| c.to_le_bytes().to_vec())
                .collect();
            print!("    Name UTF16: ");
            for b in &name_bytes {
                print!("{:02x} ", b);
            }
            println!();
        }
    }
    println!("\nTotal entries: {}", entries.len());
}
