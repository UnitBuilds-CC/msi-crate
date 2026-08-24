/// Verify cabinet data in CFB-modified MSI
fn main() {
    use std::io::Cursor;
    let path = "C:\\temp\\enc_cab_hashvelcabdotcab.msi";
    let data = std::fs::read(path).unwrap();
    let cursor = Cursor::new(&data);
    let mut comp = cfb::CompoundFile::open(cursor).unwrap();
    
    // Find and read the cabinet stream
    let entries: Vec<_> = comp.walk()
        .filter(|e| e.is_stream())
        .map(|e| e.name().to_string())
        .collect();
    
    for name in &entries {
        if name.contains("velcab") || name.contains("cab") || name.starts_with('#') {
            println!("Found cabinet stream: '{}'", name);
            let mut s = comp.open_stream(name).unwrap();
            let mut buf = Vec::new();
            std::io::Read::read_to_end(&mut s, &mut buf).unwrap();
            println!("  Size: {} bytes", buf.len());
            println!("  First 4 bytes: {:?}", std::str::from_utf8(&buf[0..4]).unwrap_or("???"));
            
            // Compare with original
            let orig = std::fs::read("C:\\temp\\good.cab").unwrap();
            if buf == orig {
                println!("  MATCH: Cabinet data matches good.cab exactly!");
            } else {
                println!("  MISMATCH: Data differs from good.cab ({} vs {} bytes)", buf.len(), orig.len());
                // Show first difference
                for i in 0..buf.len().min(orig.len()) {
                    if buf[i] != orig[i] {
                        println!("  First diff at byte {}: got {:02x}, expected {:02x}", i, buf[i], orig[i]);
                        break;
                    }
                }
            }
        }
    }
}
