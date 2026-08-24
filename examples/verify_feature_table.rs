/// Diagnostic: use msi crate to read our compiler MSI and verify Feature table data.
/// cargo run --example verify_feature_table -p velocity-msi
use std::io::Cursor;

fn main() {
    let msi_path = r"C:\Users\visse\OneDrive\Documentos\V.E.L.O.C.I.T.Y.-Installer-master\examples\sample-app\output\installer.msi";
    
    println!("=== VERIFY FEATURE TABLE ===\n");
    
    // Read MSI file
    let msi_data = std::fs::read(msi_path).expect("read MSI file");
    let cursor = Cursor::new(msi_data);
    let mut pkg = msi::Package::open(cursor).expect("open MSI");
    
    // Dump _Tables
    println!("=== _Tables ===");
    for row in pkg.select_rows(msi::Select::table("_Tables")).expect("read _Tables") {
        println!("  {}", row[0].as_str().unwrap_or("?"));
    }
    
    // Dump _Columns for Feature table
    println!("\n=== _Columns for Feature ===");
    for row in pkg.select_rows(msi::Select::table("_Columns")).expect("read _Columns") {
        let table = row[0].as_str().unwrap_or("?");
        if table == "Feature" {
            let number = row[1].as_int().unwrap_or(-1);
            let name = row[2].as_str().unwrap_or("?");
            let type_val = row[3].as_int().unwrap_or(-1);
            println!("  Col {}: Name={}, Type=0x{:04X}", number, name, type_val);
            let size = type_val & 0xFF;
            let valid = (type_val & 0x100) != 0;
            let localizable = (type_val & 0x200) != 0;
            let nonbinary = (type_val & 0x400) != 0;
            let string_bit = (type_val & 0x800) != 0;
            let nullable = (type_val & 0x1000) != 0;
            let pk = (type_val & 0x2000) != 0;
            println!("    size={}, valid={}, localizable={}, nonbinary={}, string={}, nullable={}, pk={}",
                size, valid, localizable, nonbinary, string_bit, nullable, pk);
        }
    }
    
    // Dump Feature table data
    println!("\n=== Feature table data ===");
    for row in pkg.select_rows(msi::Select::table("Feature")).expect("read Feature") {
        let feature = row[0].as_str().unwrap_or("?");
        let parent = row[1].as_str().unwrap_or("Null");
        let title = row[2].as_str().unwrap_or("Null");
        let desc = row[3].as_str().unwrap_or("Null");
        let display = row[4].as_int().map(|v| v.to_string()).unwrap_or_else(|| "Null".to_string());
        let level = row[5].as_int().unwrap_or(-9999);
        let dir = row[6].as_str().unwrap_or("Null");
        let attrs = row[7].as_int().unwrap_or(-9999);
        println!("  Feature={}, Parent={}, Title={}, Desc={}", feature, parent, title, desc);
        println!("    Display={}, Level={}, Directory_={}, Attributes={}", display, level, dir, attrs);
    }
    
    // Dump FeatureComponents
    println!("\n=== FeatureComponents (first 5) ===");
    let mut count = 0;
    for row in pkg.select_rows(msi::Select::table("FeatureComponents")).expect("read FC") {
        if count >= 5 { break; }
        let feature = row[0].as_str().unwrap_or("?");
        let component = row[1].as_str().unwrap_or("?");
        println!("  Feature={}, Component={}", feature, component);
        count += 1;
    }
    
    // Dump Component table (first 3)
    println!("\n=== Component table (first 3) ===");
    count = 0;
    for row in pkg.select_rows(msi::Select::table("Component")).expect("read Component") {
        if count >= 3 { break; }
        let comp = row[0].as_str().unwrap_or("?");
        let id = row[1].as_str().unwrap_or("Null");
        let dir = row[2].as_str().unwrap_or("?");
        let attrs = row[3].as_int().unwrap_or(-9999);
        let cond = row[4].as_str().unwrap_or("Null");
        let keypath = row[5].as_str().unwrap_or("Null");
        println!("  Component={}, ComponentId={}, Dir={}, Attrs={}, Cond={}, KeyPath={}", 
            comp, id, dir, attrs, cond, keypath);
        count += 1;
    }
    
    // Dump InstallExecuteSequence
    println!("\n=== InstallExecuteSequence ===");
    for row in pkg.select_rows(msi::Select::table("InstallExecuteSequence")).expect("read ExecSeq") {
        let action = row[0].as_str().unwrap_or("?");
        let condition = row[1].as_str().unwrap_or("Null");
        let seq = row[2].as_int().unwrap_or(-1);
        println!("  Action={}, Condition={}, Sequence={}", action, condition, seq);
    }
    
    // Dump Property table
    println!("\n=== Property table ===");
    for row in pkg.select_rows(msi::Select::table("Property")).expect("read Property") {
        let name = row[0].as_str().unwrap_or("?");
        let value = row[1].as_str().unwrap_or("Null");
        println!("  {}={}", name, value);
    }
    
    // Dump Directory table
    println!("\n=== Directory table ===");
    for row in pkg.select_rows(msi::Select::table("Directory")).expect("read Directory") {
        let dir = row[0].as_str().unwrap_or("?");
        let parent = row[1].as_str().unwrap_or("Null");
        let default_dir = row[2].as_str().unwrap_or("?");
        println!("  Dir={}, Parent={}, DefaultDir={}", dir, parent, default_dir);
    }
    
    // Dump Media table
    println!("\n=== Media table ===");
    for row in pkg.select_rows(msi::Select::table("Media")).expect("read Media") {
        let disk_id = row[0].as_int().unwrap_or(-1);
        let last_seq = row[1].as_int().unwrap_or(-1);
        let cab = row[4].as_str().unwrap_or("Null");
        println!("  DiskId={}, LastSequence={}, Cabinet={}", disk_id, last_seq, cab);
    }
    
    // Dump File table (first 3)
    println!("\n=== File table (first 3) ===");
    count = 0;
    for row in pkg.select_rows(msi::Select::table("File")).expect("read File") {
        if count >= 3 { break; }
        let file = row[0].as_str().unwrap_or("?");
        let comp = row[1].as_str().unwrap_or("?");
        let name = row[2].as_str().unwrap_or("?");
        let size = row[3].as_int().unwrap_or(-1);
        let seq = row[7].as_int().unwrap_or(-1);
        println!("  File={}, Component={}, Name={}, Size={}, Sequence={}", file, comp, name, size, seq);
        count += 1;
    }
    
    println!("\nDone.");
}
