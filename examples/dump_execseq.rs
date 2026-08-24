/// Dump streams from execseq_test.msi to verify Directory data.
use std::io::Read;

fn main() {
    let path = "execseq_test.msi";
    let file = std::fs::File::open(path).unwrap();
    let mut comp = cfb::CompoundFile::open(file).unwrap();

    let stream_entries: Vec<String> = comp.walk()
        .filter(|e| e.is_stream())
        .map(|e| e.path().to_string_lossy().to_string())
        .collect();

    drop(comp);

    let file2 = std::fs::File::open(path).unwrap();
    let mut comp2 = cfb::CompoundFile::open(file2).unwrap();

    for stream_path in &stream_entries {
        let data = {
            let mut reader = comp2.open_stream(stream_path.as_str()).unwrap();
            let mut buf = Vec::new();
            reader.read_to_end(&mut buf).unwrap();
            buf
        };

        // Identify by size and content heuristics
        let is_string_pool = data.len() >= 4 && {
            let header = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
            (header & 0xFFFF) == 1252
        };

        if is_string_pool {
            println!("=== _StringPool ({} bytes) ===", data.len());
            let entry_count = (data.len() - 4) / 4;
            println!("  Entries: {}", entry_count);
            for i in 0..entry_count {
                let off = 4 + i * 4;
                let len = u16::from_le_bytes([data[off], data[off + 1]]);
                let rc = u16::from_le_bytes([data[off + 2], data[off + 3]]);
                println!("    [{}]: len={}, refcount={}", i + 1, len, rc);
            }
        } else {
            // Check if it looks like _StringData (printable ASCII)
            let is_string_data = data.iter().take(20).all(|&b| b >= 0x20 && b < 0x7f);
            if is_string_data && data.len() > 20 {
                println!("=== _StringData ({} bytes) ===", data.len());
                let text: String = data.iter().map(|&b| b as char).collect();
                println!("  Content: {:?}", &text[..text.len().min(200)]);
            } else {
                println!("=== {} ({} bytes) ===", stream_path, data.len());
                // Hex dump
                let dump_len = data.len().min(128);
                for row in (0..dump_len).step_by(16) {
                    print!("  {:04x}: ", row);
                    for j in 0..16 {
                        if row + j < data.len() {
                            print!("{:02x} ", data[row + j]);
                        } else {
                            print!("   ");
                        }
                    }
                    print!(" |");
                    for j in 0..16 {
                        if row + j < data.len() {
                            let b = data[row + j];
                            if b >= 0x20 && b < 0x7f {
                                print!("{}", b as char);
                            } else {
                                print!(".");
                            }
                        }
                    }
                    println!("|");
                }
            }
        }
        println!();
    }

    // Now use msi crate to read Directory table rows
    println!("\n=== msi crate: Directory table data ===");
    let file3 = std::fs::File::open(path).unwrap();
    let pkg = msi::Package::open(file3).unwrap();
    for table in pkg.tables() {
        if table.name() == "Directory" {
            println!("Directory table found, {} columns", table.columns().len());
            // Read raw data
            let data = {
                let file4 = std::fs::File::open(path).unwrap();
                let mut comp3 = cfb::CompoundFile::open(file4).unwrap();
                // Find Directory stream
                let dir_path = comp3.walk()
                    .find(|e| {
                        let name = e.path().to_string_lossy();
                        // Directory stream name is encoded
                        name.contains("Directory") || {
                            // Try to identify by size: 2 rows × 3 string cols = 12 bytes
                            false
                        }
                    })
                    .map(|e| e.path().to_string_lossy().to_string());
                if let Some(p) = dir_path {
                    let mut reader = comp3.open_stream(p.as_str()).unwrap();
                    let mut buf = Vec::new();
                    reader.read_to_end(&mut buf).unwrap();
                    Some(buf)
                } else {
                    None
                }
            };
            if let Some(d) = data {
                println!("  Raw data ({} bytes): {:02x?}", d.len(), &d[..d.len().min(32)]);
            }
        }
    }
}
