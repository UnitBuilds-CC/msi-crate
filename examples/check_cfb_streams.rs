/// Check streams in cfb_cab_test MSIs
fn main() {
    use std::io::Cursor;
    for i in 1..=2 {
        let path = format!("C:\\temp\\cfb_cab_test_{}.msi", i);
        if let Ok(data) = std::fs::read(&path) {
            println!("\n=== {} ({} bytes) ===", path, data.len());
            let cursor = Cursor::new(&data);
            let comp = cfb::CompoundFile::open(cursor).unwrap();
            for entry in comp.walk() {
                if entry.is_stream() {
                    let name = entry.name();
                    println!("  Stream: '{}'", name);
                }
            }
        }
    }
}
