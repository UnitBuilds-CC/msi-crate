/// Read InstallExecuteSequence and Property tables, and check for stream existence.
use std::io::Cursor;

fn main() {
    let msi_path = r"c:\Users\visse\OneDrive\Documentos\V.E.L.O.C.I.T.Y.-Installer-master\examples\sample-app\output\installer.msi";
    let msi_data = std::fs::read(msi_path).expect("read MSI");
    
    let cursor = Cursor::new(msi_data.clone());
    let mut pkg = msi::Package::open(cursor).expect("open MSI");
    
    // InstallExecuteSequence
    println!("=== InstallExecuteSequence ===");
    for row in pkg.select_rows(msi::Select::table("InstallExecuteSequence")).expect("read") {
        let action = row[0].as_str().unwrap_or("?");
        let condition = row[1].as_str().unwrap_or("[Null]");
        let seq = row[2].as_int().unwrap_or(-1);
        println!("  {} | {} | {}", action, condition, seq);
    }
    println!();
    
    // InstallUISequence
    println!("=== InstallUISequence ===");
    for row in pkg.select_rows(msi::Select::table("InstallUISequence")).expect("read") {
        let action = row[0].as_str().unwrap_or("?");
        let condition = row[1].as_str().unwrap_or("[Null]");
        let seq = row[2].as_int().unwrap_or(-1);
        println!("  {} | {} | {}", action, condition, seq);
    }
    println!();
    
    // Property table
    println!("=== Property Table ===");
    for row in pkg.select_rows(msi::Select::table("Property")).expect("read") {
        let prop = row[0].as_str().unwrap_or("?");
        let val = row[1].as_str().unwrap_or("[Null]");
        println!("  {} = {}", prop, val);
    }
    println!();
    
    // Check for cabinet stream in raw OLE data
    println!("=== Stream name search ===");
    // Search for "Velocity" in the raw bytes
    let search = b"Velocity";
    for i in 0..msi_data.len().saturating_sub(search.len()) {
        if &msi_data[i..i+search.len()] == search {
            // Print surrounding context
            let start = i.saturating_sub(10);
            let end = (i + search.len() + 20).min(msi_data.len());
            let context: Vec<String> = msi_data[start..end].iter()
                .map(|b| format!("{:02X}", b))
                .collect();
            println!("  Found at offset {}: ...{}...", i, context.join(" "));
        }
    }
    println!();
    
    // Also search for cabinet-related stream names in UTF-16LE
    println!("=== UTF-16LE stream name search ===");
    let cab_name = "\u{0084}V\0e\0l\0o\0c\0i\0t\0y\0";
    for i in 0..msi_data.len().saturating_sub(4) {
        // Look for the \x84 prefix byte followed by 'V' in UTF-16LE
        if msi_data[i] == 0x84 && i + 1 < msi_data.len() && msi_data[i+1] == 0x00 
            && i + 2 < msi_data.len() && msi_data[i+2] == b'V' {
            let end = (i + 40).min(msi_data.len());
            let context: Vec<String> = msi_data[i..end].iter()
                .map(|b| format!("{:02X}", b))
                .collect();
            println!("  Found at offset {}: {}...", i, context.join(" "));
        }
    }
    
    // Search for "#Velocity.cab" in the data
    println!("\n=== Search for cabinet references ===");
    let search2 = b"#Velocity.cab";
    for i in 0..msi_data.len().saturating_sub(search2.len()) {
        if &msi_data[i..i+search2.len()] == search2 {
            println!("  Found '#Velocity.cab' at offset {}", i);
        }
    }
    
    // Search for "Velocity.cab" without #
    let search3 = b"Velocity.cab";
    for i in 0..msi_data.len().saturating_sub(search3.len()) {
        if &msi_data[i..i+search3.len()] == search3 {
            println!("  Found 'Velocity.cab' at offset {}", i);
        }
    }
}
