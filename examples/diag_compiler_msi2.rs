/// Diagnostic: try opening compiler MSI with msi crate to identify structural issue
use std::io::Cursor;

fn main() {
    let path = r"C:\Users\visse\OneDrive\Documentos\V.E.L.O.C.I.T.Y.-Installer-master\examples\sample-app\output\test_compiler.msi";
    
    println!("=== DIAGNOSE COMPILER MSI ===\n");
    
    let data = std::fs::read(path).unwrap();
    println!("File size: {} bytes", data.len());
    
    // Try opening with msi crate
    println!("\n--- Opening with msi crate ---");
    match msi::Package::open(Cursor::new(&data)) {
        Ok(pkg) => {
            println!("msi crate opened successfully!");
            // List tables
            let tables: Vec<_> = pkg.tables().collect();
            println!("Tables ({}):", tables.len());
            for t in &tables {
                println!("  {:?}", t.name());
            }
        }
        Err(e) => {
            println!("msi crate FAILED to open: {}", e);
        }
    }
    
    // Also try the standalone working MSI
    let standalone_path = r"C:\temp\real_cabinet_test.msi";
    if let Ok(data2) = std::fs::read(standalone_path) {
        println!("\n--- Opening standalone MSI with msi crate ---");
        match msi::Package::open(Cursor::new(&data2)) {
            Ok(pkg) => {
                println!("Standalone opened successfully!");
                let tables: Vec<_> = pkg.tables().collect();
                println!("Tables ({}):", tables.len());
                for t in &tables {
                    println!("  {:?}", t.name());
                }
            }
            Err(e) => {
                println!("Standalone FAILED: {}", e);
            }
        }
    }
}
