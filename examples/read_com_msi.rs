/// Test: Open COM-created MSI with msi crate and read tables
/// cargo run --example read_com_msi -p velocity-msi
use std::io::Cursor;

fn main() {
    println!("=== READ COM MSI TEST ===\n");

    let path = "C:\\temp\\com_full.msi";
    if !std::path::Path::new(path).exists() {
        println!("File not found: {}", path);
        return;
    }

    let data = std::fs::read(path).unwrap();
    println!("File: {} bytes", data.len());

    // Open with cfb to list streams
    {
        let comp = cfb::CompoundFile::open(Cursor::new(&data)).unwrap();
        println!("\nStreams:");
        let entries: Vec<_> = comp.walk().map(|e| (e.name().to_string(), e.is_stream())).collect();
        for (name, is_stream) in &entries {
            println!("  {} [{}]", name, if *is_stream { "S" } else { "D" });
        }
    }

    // Open with msi crate
    println!("\nOpening with msi crate...");
    let cursor = Cursor::new(data);
    match msi::Package::open(cursor) {
        Ok(mut pkg) => {
            println!("Opened OK!");

            // List tables
            let tables: Vec<String> = pkg.tables().map(|t| t.name().to_string()).collect();
            println!("Tables: {:?}", tables);

            // Read Property table
            let select = msi::Select::table("Property");
            match pkg.select_rows(select) {
                Ok(rows) => {
                    println!("\nProperty rows ({}):", rows.len());
                    for row in rows {
                        let vals: Vec<String> = (0..row.len()).map(|i| format!("{}", row[i])).collect();
                        println!("  {:?}", vals);
                    }
                }
                Err(e) => println!("Select failed: {:?}", e),
            }

            // Check SummaryInfo
            let si = pkg.summary_info();
            println!("\nSummaryInfo:");
            println!("  Title: {:?}", si.title());
            println!("  Subject: {:?}", si.subject());
            println!("  Author: {:?}", si.author());
            println!("  Codepage: {:?}", si.codepage());
            println!("  UUID: {:?}", si.uuid());
            println!("  WordCount: {:?}", si.word_count());
        }
        Err(e) => {
            println!("Open failed: {:?}", e);
        }
    }

    println!("\n=== DONE ===");
}
