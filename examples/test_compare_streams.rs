//! Compare compiler MSI with progressive test MSI (Phase 6 equivalent)
use std::io::Cursor;

fn main() {
    let compiler_path = r"C:\Users\visse\OneDrive\Documentos\V.E.L.O.C.I.T.Y.-Installer-master\examples\sample-app\output\installer.msi";
    let test_path = r"C:\Users\visse\OneDrive\Documentos\V.E.L.O.C.I.T.Y.-Installer-master\progressive_phase6.msi";

    println!("=== Compiler MSI ===");
    let compiler_data = std::fs::read(compiler_path).unwrap();
    dump_streams(&compiler_data);

    println!("\n=== Progressive Phase 6 MSI ===");
    let test_data = std::fs::read(test_path).unwrap();
    dump_streams(&test_data);

    // Compare _StringPool streams
    println!("\n=== String Pool Comparison ===");
    compare_stream(&compiler_data, &test_data, "_StringPool");
    compare_stream(&compiler_data, &test_data, "_StringData");
}

fn find_stream(msi_data: &[u8], name_contains: &str) -> Option<Vec<u8>> {
    let cursor = Cursor::new(msi_data);
    let mut comp = cfb::CompoundFile::open(cursor).ok()?;
    // Collect matching paths first to avoid borrow issues
    let paths: Vec<_> = comp.walk()
        .filter(|e| e.is_stream() && e.path().to_string_lossy().contains(name_contains))
        .map(|e| e.path().to_owned())
        .collect();
    if let Some(path) = paths.first() {
        let mut stream = comp.open_stream(path).ok()?;
        use std::io::Read;
        let mut data = Vec::new();
        stream.read_to_end(&mut data).ok()?;
        return Some(data);
    }
    None
}

fn compare_stream(a: &[u8], b: &[u8], name: &str) {
    let sa = find_stream(a, name);
    let sb = find_stream(b, name);
    match (sa, sb) {
        (Some(da), Some(db)) => {
            println!("  {} : compiler={} bytes, test={} bytes", name, da.len(), db.len());
            if da == db {
                println!("    IDENTICAL");
            } else {
                println!("    DIFFERENT!");
                // Show first difference
                for i in 0..da.len().min(db.len()) {
                    if da[i] != db[i] {
                        println!("    First diff at byte {}: compiler=0x{:02X}, test=0x{:02X}", i, da[i], db[i]);
                        // Show context
                        let start = i.saturating_sub(8);
                        let end = (i + 16).min(da.len());
                        println!("    Compiler: {:02X?}", &da[start..end]);
                        let end2 = (i + 16).min(db.len());
                        println!("    Test:     {:02X?}", &db[start..end2]);
                        break;
                    }
                }
                if da.len() != db.len() {
                    println!("    Size difference: {} vs {}", da.len(), db.len());
                }
            }
        }
        (None, _) => println!("  {} : NOT FOUND in compiler MSI", name),
        (_, None) => println!("  {} : NOT FOUND in test MSI", name),
    }
}

fn dump_streams(msi_data: &[u8]) {
    let cursor = Cursor::new(msi_data);
    match cfb::CompoundFile::open(cursor) {
        Ok(comp) => {
            let entries = comp.walk();
            let mut stream_count = 0;
            let mut total_data = 0usize;
            for entry in entries {
                if entry.is_stream() {
                    stream_count += 1;
                    let size = entry.len() as usize;
                    total_data += size;
                    let name = entry.path().to_string_lossy().to_string();
                    println!("  {:50} {} bytes", name, size);
                }
            }
            println!("  Total: {} streams, {} bytes", stream_count, total_data);
        }
        Err(e) => println!("  ERROR: {}", e),
    }
}
