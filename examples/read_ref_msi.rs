//! Read a reference MSI from the system and dump its File table _Columns entries
//! This tells us exactly what Windows Installer expects for the File table

use std::io::Cursor;

fn main() {
    // Try to find an MSI with a File table
    let ref_paths = vec![
        "C:\\Program Files\\Microsoft Office\\root\\Integration\\C2RInt.16.msi",
    ];
    
    for path in &ref_paths {
        if !std::path::Path::new(path).exists() {
            println!("Not found: {}", path);
            continue;
        }
        
        println!("=== Reading reference MSI: {} ===", path);
        let data = std::fs::read(path).unwrap();
        println!("Size: {} bytes", data.len());
        
        let cursor = Cursor::new(data);
        let mut package = match msi::Package::open(cursor) {
            Ok(p) => p,
            Err(e) => {
                println!("Failed to open: {}", e);
                continue;
            }
        };
        
        // Check if it has a File table
        println!("\n=== Tables in package ===");
        let mut has_file_table = false;
        if let Ok(rows) = package.select_rows(msi::Select::table("_Tables")) {
            for row in rows {
                // _Tables has column named "Name" not "Table"
                let name = match &row[0] {
                    msi::Value::Str(s) => s.clone(),
                    _ => continue,
                };
                if name == "File" {
                    has_file_table = true;
                }
                println!("  {}", name);
            }
        }
        
        if !has_file_table {
            println!("\nNo File table found, skipping.");
            continue;
        }
        
        // Read _Columns for File table
        println!("\n=== _Columns for File table ===");
        if let Ok(rows) = package.select_rows(msi::Select::table("_Columns")) {
            for row in rows {
                let table_name = match &row["Table"] {
                    msi::Value::Str(s) => s.clone(),
                    _ => continue,
                };
                if table_name != "File" {
                    continue;
                }
                let num = match &row["Number"] {
                    msi::Value::Int(n) => *n,
                    _ => 0,
                };
                let name = match &row["Name"] {
                    msi::Value::Str(s) => s.clone(),
                    _ => "?".to_string(),
                };
                let sql_type = match &row["Type"] {
                    msi::Value::Int(n) => *n,
                    _ => 0,
                };
                println!("  Number={} Name='{}' Type=0x{:04X}", num, name, sql_type);
                
                // Decode the type bitfield
                let size = sql_type & 0xFF;
                let valid = (sql_type & 0x100) != 0;
                let localizable = (sql_type & 0x200) != 0;
                let nonbinary = (sql_type & 0x400) != 0;
                let string = (sql_type & 0x800) != 0;
                let nullable = (sql_type & 0x1000) != 0;
                let pk = (sql_type & 0x2000) != 0;
                println!("    size={} valid={} localizable={} nonbinary={} string={} nullable={} pk={}",
                    size, valid, localizable, nonbinary, string, nullable, pk);
            }
        }
        
        // Read _Validation for File table
        println!("\n=== _Validation for File table ===");
        if let Ok(rows) = package.select_rows(msi::Select::table("_Validation")) {
            for row in rows {
                let table_name = match &row["Table"] {
                    msi::Value::Str(s) => s.clone(),
                    _ => continue,
                };
                if table_name != "File" {
                    continue;
                }
                let col = match &row["Column"] {
                    msi::Value::Str(s) => s.clone(),
                    _ => "?".to_string(),
                };
                let nullable = match &row["Nullable"] {
                    msi::Value::Str(s) => s.clone(),
                    _ => "?".to_string(),
                };
                let category = match &row["Category"] {
                    msi::Value::Str(s) => format!("'{}'", s),
                    _ => "Null".to_string(),
                };
                let set = match &row["Set"] {
                    msi::Value::Str(s) => format!("'{}'", s),
                    _ => "Null".to_string(),
                };
                let min = match &row["MinValue"] {
                    msi::Value::Int(n) => n.to_string(),
                    _ => "Null".to_string(),
                };
                let max = match &row["MaxValue"] {
                    msi::Value::Int(n) => n.to_string(),
                    _ => "Null".to_string(),
                };
                let key_table = match &row["KeyTable"] {
                    msi::Value::Str(s) => format!("'{}'", s),
                    _ => "Null".to_string(),
                };
                let key_col = match &row["KeyColumn"] {
                    msi::Value::Int(n) => n.to_string(),
                    _ => "Null".to_string(),
                };
                println!("  Col='{}' Nullable={} Cat={} Set={} Min={} Max={} KeyTable={} KeyCol={}",
                    col, nullable, category, set, min, max, key_table, key_col);
            }
        }
        
        // Read File table data
        println!("\n=== File table data ===");
        if let Ok(rows) = package.select_rows(msi::Select::table("File")) {
            let count = rows.count();
            println!("  {} rows", count);
        } else {
            println!("  (could not read - might be empty)");
        }
        
        // Read string pool info
        println!("\n=== String pool info ===");
        println!("  Codepage: {:?}", package.database_codepage());
        
        // Only process first MSI
        break;
    }
}
