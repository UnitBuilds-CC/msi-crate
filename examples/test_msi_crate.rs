//! Definitive test: Can the msi crate produce an installable MSI?
//! 
//! This test:
//! 1. Creates a minimal MSI with msi crate (Package::create)
//! 2. Repackages it as V3 using cfb
//! 3. Tests with msiexec
//! 4. If it works, we know the msi crate DATA is valid and we can use it as reference
//! 5. If it fails, the msi crate's flush path is broken

use std::io::{Cursor, Read};
use std::process::Command;

fn main() {
    println!("=== Test: msi crate minimal MSI ===\n");
    
    // Create a minimal MSI with msi crate
    let cursor = Cursor::new(Vec::new());
    let mut package = msi::Package::create(msi::PackageType::Installer, cursor).unwrap();
    
    // Set required SummaryInfo properties
    {
        let summary = package.summary_info_mut();
        summary.set_title("Installation Database".to_string());
        summary.set_subject("TestProduct".to_string());
        summary.set_author("TestCo".to_string());
        // PID 7 - Subject/Template
        // PID 9 - Revision Number (REQUIRED by msiexec)
        summary.set_revision_number("{12345678-1234-1234-1234-123456789012}".to_string());
        // PID 14 - Security (required)
        summary.set_security(405);
        // PID 15 - WordCount
        summary.set_word_count(2);
    }
    
    // Add Property table
    package.create_table("Property", vec![
        msi::Column::build("Property").primary_key().string(72),
        msi::Column::build("Value").nullable().string(255),
    ]).unwrap();
    
    package.insert_rows(msi::Insert::into("Property").rows(vec![
        vec![msi::Value::Str("ProductName".to_string()), msi::Value::Str("TestProduct".to_string())],
        vec![msi::Value::Str("ProductCode".to_string()), msi::Value::Str("{12345678-1234-1234-1234-123456789012}".to_string())],
        vec![msi::Value::Str("ProductVersion".to_string()), msi::Value::Str("1.0.0".to_string())],
        vec![msi::Value::Str("ProductLanguage".to_string()), msi::Value::Str("1033".to_string())],
        vec![msi::Value::Str("Manufacturer".to_string()), msi::Value::Str("TestCo".to_string())],
        vec![msi::Value::Str("UpgradeCode".to_string()), msi::Value::Str("{87654321-4321-4321-4321-210987654321}".to_string())],
    ])).unwrap();
    
    // Get V4 data
    let v4_cursor = package.into_inner().unwrap();
    let v4_data = v4_cursor.into_inner();
    println!("V4 MSI size: {} bytes", v4_data.len());
    
    // Write V4 for testing
    std::fs::create_dir_all("C:\\temp\\msi_crate_test").ok();
    let v4_path = "C:\\temp\\msi_crate_test\\msi_crate_v4.msi";
    std::fs::write(v4_path, &v4_data).unwrap();
    
    // Test V4 with msiexec
    println!("\n=== Testing V4 MSI ===");
    let v4_log = "C:\\temp\\msi_crate_test\\v4_install.log";
    let status = Command::new("msiexec")
        .args(&["/i", v4_path, "/qn", "/norestart", "/l*v", v4_log])
        .status();
    match status {
        Ok(s) => {
            let code = s.code().unwrap_or(-1);
            println!("V4 msiexec exit code: {} ({})", code, 
                match code {
                    0 => "SUCCESS",
                    1620 => "package not valid",
                    1613 => "package cannot be opened",
                    _ => "other error",
                });
        }
        Err(e) => println!("Failed to run msiexec: {}", e),
    }
    
    // Now repackage as V3 using cfb
    println!("\n=== Repackaging V4 → V3 ===");
    let v3_data = repackage_v4_to_v3(&v4_data);
    
    if let Some(v3) = v3_data {
        println!("V3 MSI size: {} bytes", v3.len());
        let v3_path = "C:\\temp\\msi_crate_test\\msi_crate_v3.msi";
        std::fs::write(v3_path, &v3).unwrap();
        
        // Test V3 with msiexec
        println!("\n=== Testing V3 MSI ===");
        let v3_log = "C:\\temp\\msi_crate_test\\v3_install.log";
        let status = Command::new("msiexec")
            .args(&["/i", v3_path, "/qn", "/norestart", "/l*v", v3_log])
            .status();
        match status {
            Ok(s) => {
                let code = s.code().unwrap_or(-1);
                println!("V3 msiexec exit code: {} ({})", code, 
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
        println!("\n=== Stream comparison V4 vs V3 ===");
        compare_streams(&v4_data, &v3);
    } else {
        println!("Failed to repackage V4 → V3");
    }
}

fn repackage_v4_to_v3(v4_data: &[u8]) -> Option<Vec<u8>> {
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
    
    for (path, name) in entries {
        let mut stream = v4_cf.open_stream(&path).ok()?;
        let mut data = Vec::new();
        stream.read_to_end(&mut data).ok()?;
        
        // Create stream in V3 - use std::path::Path
        let stream_path = std::path::Path::new(path.as_os_str());
        let mut v3_stream = v3_cf.create_stream(stream_path).ok()?;
        use std::io::Write;
        v3_stream.write_all(&data).ok()?;
    }
    
    // Get V3 data
    let v3_cursor = v3_cf.into_inner();
    Some(v3_cursor.into_inner())
}

fn compare_streams(v4_data: &[u8], v3_data: &[u8]) {
    let v4_streams = read_all_streams(v4_data, "V4");
    let v3_streams = read_all_streams(v3_data, "V3");
    
    println!("\nV4 streams:");
    for (name, data) in &v4_streams {
        println!("  {} ({} bytes)", name, data.len());
    }
    
    println!("\nV3 streams:");
    for (name, data) in &v3_streams {
        println!("  {} ({} bytes)", name, data.len());
    }
    
    // Compare matching streams
    println!("\n=== Stream-by-stream comparison ===");
    for (name, v4_data) in &v4_streams {
        if let Some((_, v3_data)) = v3_streams.iter().find(|(n, _)| n == name) {
            if v4_data == v3_data {
                println!("  {} - IDENTICAL ({} bytes)", name, v4_data.len());
            } else {
                println!("  {} - DIFFERENT (v4={} bytes, v3={} bytes)", name, v4_data.len(), v3_data.len());
            }
        } else {
            println!("  {} - ONLY IN V4 ({} bytes)", name, v4_data.len());
        }
    }
    
    for (name, data) in &v3_streams {
        if !v4_streams.iter().any(|(n, _)| n == name) {
            println!("  {} - ONLY IN V3 ({} bytes)", name, data.len());
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
