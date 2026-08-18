/// Test with explicit writable log path to isolate sandbox issues
use std::fs::File;
use std::io::{Cursor, Read, Write as IoWrite};
use std::path::PathBuf;

fn main() {
    let sys_path = "C:\\Windows\\Installer\\10d16cbb.msi";
    let temp = std::env::temp_dir(); // User's temp dir
    println!("Temp dir: {}", temp.display());
    
    // Read all streams from system MSI
    let file = File::open(sys_path).unwrap();
    let mut comp = cfb::CompoundFile::open(file).unwrap();
    let entries: Vec<(PathBuf, bool)> = comp.walk()
        .map(|e| (e.path().to_path_buf(), e.is_stream()))
        .collect();
    
    let mut all_streams: Vec<(String, Vec<u8>)> = Vec::new();
    for (p, is_stream) in entries {
        if !is_stream { continue; }
        let name = p.file_name().unwrap().to_string_lossy().to_string();
        let mut stream = comp.open_stream(&p).unwrap();
        let mut data = Vec::new();
        stream.read_to_end(&mut data).unwrap();
        all_streams.push((name, data));
    }
    println!("Read {} streams from system MSI", all_streams.len());
    
    // Content streams only (no signatures)
    let content_streams: Vec<_> = all_streams.iter()
        .filter(|(n, _)| !n.contains("DigitalSignature") && !n.contains("MsiDigitalSignature"))
        .collect();
    
    // Test 1: Copy original system MSI to temp and test with writable log
    let sys_copy = temp.join("sys_copy.msi");
    std::fs::copy(sys_path, &sys_copy).unwrap();
    println!("\n=== Test 1: Original system MSI (copy to temp) ===");
    test_msiexec(sys_copy.to_str().unwrap(), temp.join("sys_copy.log").to_str().unwrap());
    
    // Test 2: cfb roundtrip without signatures, writable log
    println!("\n=== Test 2: cfb roundtrip no-sig (writable log) ===");
    {
        let cursor = Cursor::new(Vec::new());
        let mut out = cfb::OpenOptions::new().create_with(cursor).unwrap();
        for (name, data) in &content_streams {
            let path = format!("/{}", name);
            let mut s = out.create_stream(&path).unwrap();
            s.write_all(data).unwrap();
        }
        let cursor = out.into_inner();
        let cfb_data = cursor.into_inner();
        let out_path = temp.join("cfb_nosig.msi");
        std::fs::write(&out_path, &cfb_data).unwrap();
        println!("Written {} bytes", cfb_data.len());
        test_msiexec(out_path.to_str().unwrap(), temp.join("cfb_nosig.log").to_str().unwrap());
    }
    
    // Test 3: Our OLE writer without signatures, writable log
    println!("\n=== Test 3: Our OLE no-sig (writable log) ===");
    {
        let ole_streams: Vec<velocity_msi::ole::OleStream> = content_streams.iter()
            .map(|(name, data)| velocity_msi::ole::OleStream {
                name: name.clone(),
                data: data.clone(),
            })
            .collect();
        let our_data = velocity_msi::ole::build_ole_file(&ole_streams);
        let out_path = temp.join("our_nosig.msi");
        std::fs::write(&out_path, &our_data).unwrap();
        println!("Written {} bytes", our_data.len());
        test_msiexec(out_path.to_str().unwrap(), temp.join("our_nosig.log").to_str().unwrap());
    }
    
    // Test 4: Our generated MSI (from MsiBuilder), writable log
    println!("\n=== Test 4: Our generated MSI (writable log) ===");
    {
        let mut builder = velocity_msi::MsiBuilder::new();
        builder.set_title("Velocity Test");
        builder.set_author("Velocity");
        builder.set_template("Intel", 1033);
        builder.create_table(
            "Property",
            vec![
                velocity_msi::Column::build("Property").string(72).primary_key().build(),
                velocity_msi::Column::build("Value").string(255).nullable().build(),
            ],
        ).unwrap();
        builder.insert_rows("Property", vec![
            vec![velocity_msi::Value::from("ProductName"), velocity_msi::Value::from("Test")],
            vec![velocity_msi::Value::from("ProductVersion"), velocity_msi::Value::from("1.0")],
            vec![velocity_msi::Value::from("Manufacturer"), velocity_msi::Value::from("Test")],
            vec![velocity_msi::Value::from("ProductCode"), velocity_msi::Value::from("{12345678-1234-1234-1234-123456789012}")],
            vec![velocity_msi::Value::from("UpgradeCode"), velocity_msi::Value::from("{87654321-4321-4321-4321-210987654321}")],
        ]).unwrap();
        let our_data = builder.build().unwrap();
        let out_path = temp.join("velocity_test.msi");
        std::fs::write(&out_path, &our_data).unwrap();
        println!("Written {} bytes", our_data.len());
        test_msiexec(out_path.to_str().unwrap(), temp.join("velocity_test.log").to_str().unwrap());
    }
    
    // Test 5: Try msiexec /a (administrative install) which might not need caching
    println!("\n=== Test 5: msiexec /a (admin install) on cfb no-sig ===");
    {
        let msi_path = temp.join("cfb_nosig.msi");
        let log_path = temp.join("admin_install.log");
        let _ = std::fs::remove_file(&log_path);
        let status = std::process::Command::new("msiexec.exe")
            .args(&["/a", msi_path.to_str().unwrap(), "/qn", "/l*v", log_path.to_str().unwrap()])
            .status()
            .unwrap();
        let code = status.code().unwrap_or(-1);
        println!("  Admin install result: code={}", code);
        if let Ok(log) = std::fs::read_to_string(&log_path) {
            for line in log.lines() {
                if line.contains("error") || line.contains("Error") || line.contains("Access database") 
                    || line.contains("Could not") || line.contains("return value") {
                    println!("    LOG: {}", line.trim());
                }
            }
        } else {
            println!("  (no log)");
        }
    }
}

fn test_msiexec(msi_path: &str, log_path: &str) {
    let _ = std::fs::remove_file(log_path);
    let status = std::process::Command::new("msiexec.exe")
        .args(&["/i", msi_path, "/qn", "/l*v", log_path])
        .status()
        .unwrap();
    let code = status.code().unwrap_or(-1);
    
    if let Ok(log) = std::fs::read_to_string(log_path) {
        if log.contains("Access database") {
            println!("  Result: SUCCESS - Database accessed! (code={})", code);
        } else if log.contains("1620") {
            println!("  Result: Error 1620 (invalid package)");
        } else if log.contains("1619") {
            println!("  Result: Error 1619 (package not accessible)");
        } else if log.contains("1603") {
            println!("  Result: Error 1603 (fatal error)");
        } else if log.contains("1622") {
            println!("  Result: Error 1622 (log file error)");
        } else {
            println!("  Result: code={}", code);
        }
        // Show key log lines
        for line in log.lines() {
            if line.contains("error") || line.contains("Error") || line.contains("return value 3") 
                || line.contains("Access database") || line.contains("Could not open")
                || line.contains("MSI_OpenDatabase") {
                println!("    LOG: {}", line.trim());
            }
        }
    } else {
        println!("  Result: code={} (no log)", code);
    }
}
