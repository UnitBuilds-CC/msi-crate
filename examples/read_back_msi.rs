/// Open our MSI with the msi crate and dump all table data.
use std::io::Cursor;

fn val_str(v: &msi::Value) -> String {
    match v {
        msi::Value::Null => "[Null]".into(),
        msi::Value::Str(s) => format!("\"{}\"", s),
        msi::Value::Int(i) => format!("{}", i),
        _ => "?".into(),
    }
}

fn main() {
    let msi_path = r"c:\Users\visse\OneDrive\Documentos\V.E.L.O.C.I.T.Y.-Installer-master\examples\sample-app\output\installer.msi";
    
    println!("Opening MSI: {}", msi_path);
    let msi_data = std::fs::read(msi_path).expect("read MSI");
    println!("Size: {} bytes\n", msi_data.len());
    
    let cursor = Cursor::new(msi_data);
    let mut pkg = match msi::Package::open(cursor) {
        Ok(p) => p,
        Err(e) => { println!("FAILED to open: {:?}", e); return; }
    };
    
    println!("MSI opened successfully!\n");
    
    // List all tables
    println!("=== TABLE LIST ===");
    for t in pkg.tables() {
        println!("  {}", t.name());
    }
    println!();
    
    // _Tables
    println!("=== _TABLES ===");
    for row in pkg.select_rows(msi::Select::table("_Tables")).expect("read") {
        println!("  {}", row[0].as_str().unwrap_or("?"));
    }
    println!();
    
    // Component table
    println!("=== COMPONENT TABLE ===");
    let mut comp_count = 0;
    for row in pkg.select_rows(msi::Select::table("Component")).expect("read") {
        println!("  {}, {}, {}, {}, {}, {}",
            val_str(&row[0]), val_str(&row[1]), val_str(&row[2]),
            val_str(&row[3]), val_str(&row[4]), val_str(&row[5]));
        comp_count += 1;
    }
    println!("  ({} rows)\n", comp_count);
    
    // File table (first 3)
    println!("=== FILE TABLE (first 3) ===");
    let mut file_count = 0;
    for row in pkg.select_rows(msi::Select::table("File")).expect("read") {
        if file_count < 3 {
            println!("  {}, {}, {}, {}, {}, {}, {}, {}",
                val_str(&row[0]), val_str(&row[1]), val_str(&row[2]),
                val_str(&row[3]), val_str(&row[4]), val_str(&row[5]),
                val_str(&row[6]), val_str(&row[7]));
        }
        file_count += 1;
    }
    println!("  ({} total rows)\n", file_count);
    
    // Feature table
    println!("=== FEATURE TABLE ===");
    for row in pkg.select_rows(msi::Select::table("Feature")).expect("read") {
        println!("  {}, {}, {}, {}, {}, {}, {}, {}",
            val_str(&row[0]), val_str(&row[1]), val_str(&row[2]),
            val_str(&row[3]), val_str(&row[4]), val_str(&row[5]),
            val_str(&row[6]), val_str(&row[7]));
    }
    println!();
    
    // FeatureComponents (first 5)
    println!("=== FEATURECOMPONENTS (first 5) ===");
    let mut fc_count = 0;
    for row in pkg.select_rows(msi::Select::table("FeatureComponents")).expect("read") {
        if fc_count < 5 {
            println!("  {}, {}", val_str(&row[0]), val_str(&row[1]));
        }
        fc_count += 1;
    }
    println!("  ({} total rows)\n", fc_count);
    
    // Media table
    println!("=== MEDIA TABLE ===");
    for row in pkg.select_rows(msi::Select::table("Media")).expect("read") {
        println!("  {}, {}, {}, {}, {}, {}",
            val_str(&row[0]), val_str(&row[1]), val_str(&row[2]),
            val_str(&row[3]), val_str(&row[4]), val_str(&row[5]));
    }
    println!();
    
    // Directory table
    println!("=== DIRECTORY TABLE ===");
    for row in pkg.select_rows(msi::Select::table("Directory")).expect("read") {
        println!("  {}, {}, {}", val_str(&row[0]), val_str(&row[1]), val_str(&row[2]));
    }
    println!();
    
    // _Columns for Component (show type in hex)
    println!("=== _COLUMNS for Component ===");
    for row in pkg.select_rows(msi::Select::table("_Columns")).expect("read") {
        let table = row[0].as_str().unwrap_or("?");
        if table == "Component" {
            let number = row[1].as_int().unwrap_or(-1);
            let name = row[2].as_str().unwrap_or("?");
            let type_val = row[3].as_int().unwrap_or(-1);
            println!("  Col {}: {} = 0x{:04X}", number, name, type_val);
        }
    }
    println!();
    
    // _Validation for Component
    println!("=== _VALIDATION for Component ===");
    for row in pkg.select_rows(msi::Select::table("_Validation")).expect("read") {
        let table = row[0].as_str().unwrap_or("?");
        if table == "Component" {
            let col = row[1].as_str().unwrap_or("?");
            let nullable = row[2].as_str().unwrap_or("?");
            let category = row[7].as_str().unwrap_or("[Null]");
            println!("  Component.{}: Nullable={}, Category={}", col, nullable, category);
        }
    }
}
