/// Diagnostic: dump _Columns for Component table from our MSI
/// cargo run --example dump_component_schema -p velocity-msi
use std::io::Cursor;

fn main() {
    let msi_path = r"C:\Users\visse\OneDrive\Documentos\V.E.L.O.C.I.T.Y.-Installer-master\examples\sample-app\output\installer.msi";
    let msi_data = std::fs::read(msi_path).expect("read MSI");
    let cursor = Cursor::new(msi_data);
    let mut pkg = msi::Package::open(cursor).expect("open MSI");
    
    // Dump _Columns for Component table
    println!("=== _Columns for Component ===");
    for row in pkg.select_rows(msi::Select::table("_Columns")).expect("read _Columns") {
        let table = row[0].as_str().unwrap_or("?");
        if table == "Component" {
            let number = row[1].as_int().unwrap_or(-1);
            let name = row[2].as_str().unwrap_or("?");
            let type_val = row[3].as_int().unwrap_or(-1);
            let size = type_val & 0xFF;
            let valid = (type_val & 0x100) != 0;
            let localizable = (type_val & 0x200) != 0;
            let nonbinary = (type_val & 0x400) != 0;
            let string_bit = (type_val & 0x800) != 0;
            let nullable = (type_val & 0x1000) != 0;
            let pk = (type_val & 0x2000) != 0;
            println!("  Col {}: Name={}, Type=0x{:04X} (size={}, valid={}, loc={}, nonbin={}, str={}, null={}, pk={})", 
                number, name, type_val, size, valid, localizable, nonbinary, string_bit, nullable, pk);
        }
    }
    
    // Also dump _Columns for Feature table for comparison
    println!("\n=== _Columns for Feature ===");
    for row in pkg.select_rows(msi::Select::table("_Columns")).expect("read _Columns") {
        let table = row[0].as_str().unwrap_or("?");
        if table == "Feature" {
            let number = row[1].as_int().unwrap_or(-1);
            let name = row[2].as_str().unwrap_or("?");
            let type_val = row[3].as_int().unwrap_or(-1);
            println!("  Col {}: Name={}, Type=0x{:04X}", number, name, type_val);
        }
    }
    
    // Dump ALL _Columns entries to see the full picture
    println!("\n=== ALL _Columns entries ===");
    for row in pkg.select_rows(msi::Select::table("_Columns")).expect("read _Columns") {
        let table = row[0].as_str().unwrap_or("?");
        let number = row[1].as_int().unwrap_or(-1);
        let name = row[2].as_str().unwrap_or("?");
        let type_val = row[3].as_int().unwrap_or(-1);
        println!("  {}.{}: Type=0x{:04X}", table, name, type_val);
    }
    
    // Also check the _Validation for Component
    println!("\n=== _Validation for Component ===");
    for row in pkg.select_rows(msi::Select::table("_Validation")).expect("read _Validation") {
        let table = row[0].as_str().unwrap_or("?");
        if table == "Component" {
            let col = row[1].as_str().unwrap_or("?");
            let nullable = row[2].as_str().unwrap_or("?");
            let category = row[7].as_str().unwrap_or("Null");
            println!("  Component.{}: Nullable={}, Category={}", col, nullable, category);
        }
    }
    
    // Check if there's a MsiAssembly or MsiAssemblyName table
    println!("\n=== Checking for special tables ===");
    for row in pkg.select_rows(msi::Select::table("_Tables")).expect("read _Tables") {
        let name = row[0].as_str().unwrap_or("?");
        if name.starts_with("Msi") || name.starts_with("_") {
            println!("  {}", name);
        }
    }
}
