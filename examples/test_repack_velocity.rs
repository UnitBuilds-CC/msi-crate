//! Test: Repackage velocity-msi output with cfb to isolate OLE vs data issue
//! 
//! If velocity-msi's OLE writer has a bug, repackaging with cfb should fix it.
//! If the data is the issue, repackaging won't help.

use std::io::{Cursor, Read, Write};
use std::process::Command;
use velocity_msi::{MsiBuilder, Column, Value};

fn main() {
    println!("=== Test: velocity-msi repackaged with cfb ===\n");
    
    // Create MSI with velocity-msi
    let mut builder = MsiBuilder::new();
    builder.set_title("Test");
    builder.set_author("TestCo");
    
    // Property table
    builder.create_table("Property", vec![
        Column::build("Property").string(72).primary_key().build(),
        Column::build("Value").string(255).nullable().build(),
    ]).unwrap();
    builder.insert_rows("Property", vec![
        vec![Value::from("ProductName"), Value::from("TestProduct")],
        vec![Value::from("ProductCode"), Value::from("{12345678-1234-1234-1234-123456789012}")],
        vec![Value::from("ProductVersion"), Value::from("1.0.0")],
        vec![Value::from("ProductLanguage"), Value::from("1033")],
        vec![Value::from("Manufacturer"), Value::from("TestCo")],
        vec![Value::from("UpgradeCode"), Value::from("{87654321-4321-4321-4321-210987654321}")],
    ]).unwrap();
    
    let velocity_data = builder.build().unwrap();
    println!("velocity-msi size: {} bytes", velocity_data.len());
    
    // Write original
    std::fs::create_dir_all("C:\\temp\\repack_test").ok();
    let orig_path = "C:\\temp\\repack_test\\velocity_original.msi";
    std::fs::write(orig_path, &velocity_data).unwrap();
    
    // Test original with msiexec
    println!("\n=== Testing original velocity-msi ===");
    let orig_log = "C:\\temp\\repack_test\\original_install.log";
    let status = Command::new("msiexec")
        .args(&["/i", orig_path, "/qn", "/norestart", "/l*v", orig_log])
        .status();
    match status {
        Ok(s) => {
            let code = s.code().unwrap_or(-1);
            println!("Original msiexec exit code: {} ({})", code, 
                match code {
                    0 => "SUCCESS",
                    1620 => "package not valid",
                    1613 => "package cannot be opened",
                    _ => "other error",
                });
        }
        Err(e) => println!("Failed to run msiexec: {}", e),
    }
    
    // Repackage with cfb (V4 → V3)
    println!("\n=== Repackaging velocity-msi V4 → V3 with cfb ===");
    let repackaged = repackage_with_cfb(&velocity_data);
    
    if let Some(repack_data) = repackaged {
        println!("Repackaged size: {} bytes", repack_data.len());
        let repack_path = "C:\\temp\\repack_test\\velocity_repackaged.msi";
        std::fs::write(repack_path, &repack_data).unwrap();
        
        // Test repackaged with msiexec
        println!("\n=== Testing repackaged velocity-msi ===");
        let repack_log = "C:\\temp\\repack_test\\repack_install.log";
        let status = Command::new("msiexec")
            .args(&["/i", repack_path, "/qn", "/norestart", "/l*v", repack_log])
            .status();
        match status {
            Ok(s) => {
                let code = s.code().unwrap_or(-1);
                println!("Repackaged msiexec exit code: {} ({})", code, 
                    match code {
                        0 => "SUCCESS",
                        1620 => "package not valid",
                        1613 => "package cannot be opened",
                        _ => "other error",
                    });
            }
            Err(e) => println!("Failed to run msiexec: {}", e),
        }
        
        // Compare streams
        println!("\n=== Stream comparison ===");
        compare_streams(&velocity_data, &repack_data);
    } else {
        println!("Failed to repackage");
    }
}

fn repackage_with_cfb(v4_data: &[u8]) -> Option<Vec<u8>> {
    // Read V4 with cfb
    let v4_cursor = Cursor::new(v4_data.to_vec());
    let mut v4_cf = cfb::CompoundFile::open(v4_cursor).ok()?;
    
    // Create V3 with cfb
    let v3_cursor = Cursor::new(Vec::new());
    let mut v3_cf = cfb::CompoundFile::create_with_version(cfb::Version::V3, v3_cursor).ok()?;
    
    // Copy all streams
    let entries: Vec<_> = v4_cf.walk()
        .filter(|e| e.is_stream())
        .map(|e| (e.path().to_owned(), e.name().to_owned()))
        .collect();
    
    for (path, _name) in entries {
        let mut stream = v4_cf.open_stream(&path).ok()?;
        let mut data = Vec::new();
        stream.read_to_end(&mut data).ok()?;
        
        // Create stream in V3
        let stream_path = std::path::Path::new(path.as_os_str());
        let mut v3_stream = v3_cf.create_stream(stream_path).ok()?;
        v3_stream.write_all(&data).ok()?;
    }
    
    // Get V3 data
    let v3_cursor = v3_cf.into_inner();
    Some(v3_cursor.into_inner())
}

fn compare_streams(orig_data: &[u8], repack_data: &[u8]) {
    let orig_streams = read_all_streams(orig_data, "original");
    let repack_streams = read_all_streams(repack_data, "repackaged");
    
    println!("\nOriginal streams:");
    for (name, data) in &orig_streams {
        println!("  {} ({} bytes)", name, data.len());
    }
    
    println!("\nRepackaged streams:");
    for (name, data) in &repack_streams {
        println!("  {} ({} bytes)", name, data.len());
    }
    
    // Compare matching streams
    println!("\n=== Stream-by-stream comparison ===");
    for (name, orig_data) in &orig_streams {
        if let Some((_, repack_data)) = repack_streams.iter().find(|(n, _)| n == name) {
            if orig_data == repack_data {
                println!("  {} - IDENTICAL ({} bytes)", name, orig_data.len());
            } else {
                println!("  {} - DIFFERENT (orig={} bytes, repack={} bytes)", name, orig_data.len(), repack_data.len());
            }
        } else {
            println!("  {} - ONLY IN ORIGINAL ({} bytes)", name, orig_data.len());
        }
    }
    
    for (name, data) in &repack_streams {
        if !orig_streams.iter().any(|(n, _)| n == name) {
            println!("  {} - ONLY IN REPACKAGED ({} bytes)", name, data.len());
        }
    }
}

fn read_all_streams(data: &[u8], label: &str) -> Vec<(String, Vec<u8>)> {
    let cursor = Cursor::new(data.to_vec());
    let mut cf = match cfb::CompoundFile::open(cursor) {
        Ok(cf) => cf,
        Err(e) => {
            println!("Failed to open {} as CFB: {}", label, e);
            return Vec::new();
        }
    };
    
    let mut streams = Vec::new();
    let paths: Vec<_> = cf.walk()
        .filter_map(|e| if e.is_stream() { Some((e.path().to_owned(), e.name().to_owned())) } else { None })
        .collect();
    
    for (path, name) in paths {
        let mut stream = match cf.open_stream(&path) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let mut data = Vec::new();
        stream.read_to_end(&mut data).unwrap_or_default();
        
        if name.starts_with('\u{0005}') {
            streams.push((format!("\\u0005{}", &name[1..]), data));
        } else {
            streams.push((name, data));
        }
    }
    
    streams.sort_by(|a, b| a.0.cmp(&b.0));
    streams
}
