/// Read actual _StringData from execseq_test.msi and decode pool.
use std::io::Read;

fn main() {
    let path = "execseq_test.msi";

    // First pass: collect stream paths
    let file = std::fs::File::open(path).unwrap();
    let comp = cfb::CompoundFile::open(file).unwrap();
    let stream_entries: Vec<String> = comp.walk()
        .filter(|e| e.is_stream())
        .map(|e| e.path().to_string_lossy().to_string())
        .collect();
    drop(comp);

    // Second pass: read streams
    let file2 = std::fs::File::open(path).unwrap();
    let mut comp2 = cfb::CompoundFile::open(file2).unwrap();

    let mut pool_data: Option<Vec<u8>> = None;
    let mut string_data: Option<Vec<u8>> = None;

    for stream_path in &stream_entries {
        let mut r = comp2.open_stream(stream_path.as_str()).unwrap();
        let mut buf = Vec::new();
        r.read_to_end(&mut buf).unwrap();
        let size = buf.len();

        if size >= 4 {
            let header = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]);
            if (header & 0xFFFF) == 1252 && (size - 4) % 4 == 0 {
                let entry_count = (size - 4) / 4;
                if entry_count > 10 {
                    println!("Found _StringPool: {} entries, {} bytes", entry_count, size);
                    pool_data = Some(buf);
                    continue;
                }
            }
        }
        if size > 100 && buf.iter().take(50).all(|&b| b >= 0x20 && b < 0x7f) {
            println!("Found _StringData: {} bytes", size);
            string_data = Some(buf);
        }
    }

    if let (Some(pool), Some(data)) = (pool_data, string_data) {
        let entry_count = (pool.len() - 4) / 4;
        let mut offset = 0usize;

        println!("\n=== Decoded String Pool ===");
        let mut pool_strings: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
        for i in 0..entry_count {
            let off = 4 + i * 4;
            let len = u16::from_le_bytes([pool[off], pool[off + 1]]) as usize;

            if offset + len <= data.len() {
                let s = std::str::from_utf8(&data[offset..offset + len])
                    .unwrap_or("<invalid utf8>");
                println!("  Pool {}: len={:3} {:?}", i + 1, len, s);
                pool_strings.entry(s.to_string()).or_insert(i as u32 + 1);
                offset += len;
            } else {
                println!("  Pool {}: len={:3} <TRUNCATED at {}/{}>", i + 1, len, offset, data.len());
                break;
            }
        }

        println!("\n=== Directory table pool IDs ===");
        for name in &["INSTALLDIR", "TARGETDIR", "SourceDir", "VelTest"] {
            match pool_strings.get(*name) {
                Some(id) => println!("  {:?} = pool {} (0x{:04x})", name, id, id),
                None => println!("  {:?} = NOT FOUND", name),
            }
        }

        // Show actual Directory binary vs expected
        println!("\n=== Actual Directory binary from dump ===");
        println!("  0e 00 22 00 22 00 00 00 29 00 21 00");
        println!("  col0=[0x000e, 0x0022] col1=[0x0022, 0x0000] col2=[0x0029, 0x0021]");
        // What strings are at these pool IDs?
        let mut id_to_str: std::collections::HashMap<u32, String> = std::collections::HashMap::new();
        offset = 0;
        for i in 0..entry_count {
            let off = 4 + i * 4;
            let len = u16::from_le_bytes([pool[off], pool[off + 1]]) as usize;
            if offset + len <= data.len() {
                let s = std::str::from_utf8(&data[offset..offset + len]).unwrap_or("").to_string();
                id_to_str.insert(i as u32 + 1, s);
                offset += len;
            }
        }
        println!("\n=== Strings at actual Directory pool IDs ===");
        for id in &[0x0eu32, 0x22, 0x29, 0x21] {
            match id_to_str.get(id) {
                Some(s) => println!("  Pool {} (0x{:04x}): {:?}", id, id, s),
                None => println!("  Pool {} (0x{:04x}): NOT FOUND", id, id),
            }
        }
    }
}
