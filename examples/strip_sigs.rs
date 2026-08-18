/// Test 2.0: Strip digital signature streams and test with cfb + our OLE
/// This isolates whether the issue is digital signatures or content.
use std::fs::File;
use std::io::{Cursor, Read, Write as IoWrite};
use std::path::PathBuf;

fn main() {
    let sys_path = "C:\\Windows\\Installer\\10d16cbb.msi";
    
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
    
    // Separate signature streams from content streams
    let sig_streams: Vec<_> = all_streams.iter()
        .filter(|(n, _)| n.contains("DigitalSignature") || n.contains("MsiDigitalSignature"))
        .collect();
    let content_streams: Vec<_> = all_streams.iter()
        .filter(|(n, _)| !n.contains("DigitalSignature") && !n.contains("MsiDigitalSignature"))
        .collect();
    
    println!("Signature streams: {}", sig_streams.len());
    for (n, d) in &sig_streams {
        println!("  '{}' ({} bytes)", n, d.len());
    }
    println!("Content streams: {}", content_streams.len());
    
    // Test 1: cfb roundtrip WITHOUT signature streams
    println!("\n=== Test 1: cfb roundtrip WITHOUT signatures ===");
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
        std::fs::write("C:\\Windows\\Temp\\cfb_nosig.msi", &cfb_data).unwrap();
        println!("Written {} bytes ({} streams)", cfb_data.len(), content_streams.len());
    }
    test_msiexec("C:\\Windows\\Temp\\cfb_nosig.msi");
    
    // Test 2: Our OLE writer WITHOUT signature streams
    println!("\n=== Test 2: Our OLE writer WITHOUT signatures ===");
    {
        let ole_streams: Vec<velocity_msi::ole::OleStream> = content_streams.iter()
            .map(|(name, data)| velocity_msi::ole::OleStream {
                name: name.clone(),
                data: data.clone(),
            })
            .collect();
        let our_data = velocity_msi::ole::build_ole_file(&ole_streams);
        std::fs::write("C:\\Windows\\Temp\\our_nosig.msi", &our_data).unwrap();
        println!("Written {} bytes ({} streams)", our_data.len(), content_streams.len());
    }
    test_msiexec("C:\\Windows\\Temp\\our_nosig.msi");
    
    // Test 3: cfb with ONLY essential streams (no signatures, no cabinets)
    println!("\n=== Test 3: cfb with ONLY essential system streams ===");
    {
        let cursor = Cursor::new(Vec::new());
        let mut out = cfb::OpenOptions::new().create_with(cursor).unwrap();
        let essential_names: Vec<&str> = vec!["\u{0005}SummaryInformation"];
        let mut count = 0;
        for (name, data) in &content_streams {
            let is_essential = essential_names.iter().any(|&n| name == n)
                || (name.encode_utf16().next() == Some(0x4840) && data.len() < 100_000);
            if !is_essential { continue; }
            
            let path = format!("/{}", name);
            let mut s = out.create_stream(&path).unwrap();
            s.write_all(data).unwrap();
            count += 1;
        }
        let cursor = out.into_inner();
        let cfb_data = cursor.into_inner();
        std::fs::write("C:\\Windows\\Temp\\cfb_essential2.msi", &cfb_data).unwrap();
        println!("Written {} bytes ({} streams)", cfb_data.len(), count);
    }
    test_msiexec("C:\\Windows\\Temp\\cfb_essential2.msi");
    
    // Test 4: Our OLE writer with ONLY essential system streams
    println!("\n=== Test 4: Our OLE with ONLY essential system streams ===");
    {
        let essential_names: Vec<&str> = vec!["\u{0005}SummaryInformation"];
        let mut count = 0;
        let ole_streams: Vec<velocity_msi::ole::OleStream> = content_streams.iter()
            .filter(|(name, data)| {
                let is_essential = essential_names.iter().any(|&n| name == n)
                    || (name.encode_utf16().next() == Some(0x4840) && data.len() < 100_000);
                if is_essential { count += 1; }
                is_essential
            })
            .map(|(name, data)| velocity_msi::ole::OleStream {
                name: name.clone(),
                data: data.clone(),
            })
            .collect();
        let our_data = velocity_msi::ole::build_ole_file(&ole_streams);
        std::fs::write("C:\\Windows\\Temp\\our_essential2.msi", &our_data).unwrap();
        println!("Written {} bytes ({} streams)", our_data.len(), count);
    }
    test_msiexec("C:\\Windows\\Temp\\our_essential2.msi");
    
    // Test 5: Verify original system MSI still works
    println!("\n=== Test 5: Original system MSI (control) ===");
    test_msiexec(sys_path);
}

fn test_msiexec(path: &str) {
    let log_path = format!("{}.log", path);
    let _ = std::fs::remove_file(&log_path);
    let status = std::process::Command::new("msiexec.exe")
        .args(&["/i", path, "/qn", "/l*v", &log_path])
        .status()
        .unwrap();
    let code = status.code().unwrap_or(-1);
    
    if let Ok(log) = std::fs::read_to_string(&log_path) {
        if log.contains("Access database") {
            println!("  Result: SUCCESS - Database accessed! (code={})", code);
        } else if log.contains("1620") {
            println!("  Result: Error 1620 (invalid package)");
        } else if log.contains("1619") {
            println!("  Result: Error 1619 (package not accessible)");
        } else if log.contains("1603") {
            println!("  Result: Error 1603 (fatal error during installation)");
        } else {
            println!("  Result: code={}", code);
        }
        // Show key log lines
        if let Ok(log) = std::fs::read_to_string(&log_path) {
            for line in log.lines() {
                if line.contains("error") || line.contains("Error") || line.contains("return value 3") 
                    || line.contains("Access database") || line.contains("Could not open") {
                    println!("    LOG: {}", line.trim());
                }
            }
        }
    } else {
        println!("  Result: code={} (no log)", code);
    }
}
