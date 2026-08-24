/// Verify the V3 CFB files created by cfb crate
/// cargo run --example verify_v3 -p velocity-msi
use std::io::Cursor;

fn main() {
    println!("=== VERIFY V3 CFB ===\n");

    // Check convert_test.msi
    let path = "C:\\temp\\convert_test.msi";
    match std::fs::read(path) {
        Ok(data) => {
            println!("--- {} ({} bytes) ---", path, data.len());
            let mut comp = cfb::CompoundFile::open(Cursor::new(&data)).unwrap();
            let root = comp.root_entry();
            println!("Root CLSID: {}", root.clsid());
            println!("Root name: {:?}", root.name());

            let expected_clsid = uuid::Uuid::from_bytes([
                0x00, 0x0C, 0x10, 0x84, 0x00, 0x00, 0x00, 0x00,
                0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46,
            ]);
            println!("Expected MSI CLSID: {}", expected_clsid);
            println!("CLSID match: {}", root.clsid() == &expected_clsid);

            // Try to open with msi crate
            println!("\nTrying to open with msi crate...");
            match msi::Package::open(Cursor::new(&data)) {
                Ok(pkg) => {
                    println!("msi crate opened OK!");
                    println!("Package type: {:?}", pkg.package_type());
                    println!("Tables:");
                    for table in pkg.tables() {
                        println!("  {} ({} cols)", table.name(), table.columns().len());
                    }
                    if let Some(si) = Some(pkg.summary_info()) {
                        println!("SummaryInfo title: {:?}", si.title());
                        println!("SummaryInfo author: {:?}", si.author());
                        println!("SummaryInfo codepage: {:?}", si.codepage());
                    }
                }
                Err(e) => println!("msi crate open FAILED: {:?}", e),
            }
        }
        Err(_) => println!("File not found: {}", path),
    }

    // Also check velocity-msi output
    println!("\n\n");
    let path2 = "C:\\temp\\velocity_test.msi";
    match std::fs::read(path2) {
        Ok(data) => {
            println!("--- {} ({} bytes) ---", path2, data.len());
            let mut comp = cfb::CompoundFile::open(Cursor::new(&data)).unwrap();
            let root = comp.root_entry();
            println!("Root CLSID: {}", root.clsid());
            let expected_clsid = uuid::Uuid::from_bytes([
                0x00, 0x0C, 0x10, 0x84, 0x00, 0x00, 0x00, 0x00,
                0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46,
            ]);
            println!("CLSID match: {}", root.clsid() == &expected_clsid);

            // Try to open with msi crate
            println!("\nTrying to open with msi crate...");
            match msi::Package::open(Cursor::new(&data)) {
                Ok(pkg) => {
                    println!("msi crate opened OK!");
                    println!("Database codepage: {:?}", pkg.database_codepage());
                }
                Err(e) => println!("msi crate open FAILED: {:?}", e),
            }
        }
        Err(_) => println!("File not found: {}", path2),
    }

    println!("\n=== DONE ===");
}
