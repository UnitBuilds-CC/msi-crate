/// Resave system MSI with our OLE writer and test from accessible path
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use velocity_msi::ole;

fn main() {
    let sys_path = "C:\\Windows\\Installer\\10d16cbb.msi";
    
    // Read all streams from system MSI
    let file = File::open(sys_path).unwrap();
    let mut comp = cfb::CompoundFile::open(file).unwrap();
    let entries: Vec<(PathBuf, bool)> = comp.walk()
        .map(|e| (e.path().to_path_buf(), e.is_stream()))
        .collect();
    
    let mut streams: Vec<ole::OleStream> = Vec::new();
    for (p, is_stream) in entries {
        if !is_stream { continue; }
        let name = p.file_name().unwrap().to_string_lossy().to_string();
        let mut stream = comp.open_stream(&p).unwrap();
        let mut data = Vec::new();
        stream.read_to_end(&mut data).unwrap();
        streams.push(ole::OleStream { name, data });
    }
    println!("Read {} streams from system MSI", streams.len());
    
    // Test 1: Rebuild ALL streams with our OLE writer
    println!("\n=== Test 1: Full resave (all streams) ===");
    let rebuilt = ole::build_ole_file(&streams);
    std::fs::write("C:\\Windows\\Temp\\resave_all.msi", &rebuilt).unwrap();
    println!("Written {} bytes", rebuilt.len());
    test_msiexec("C:\\Windows\\Temp\\resave_all.msi");
    
    // Test 2: Rebuild WITHOUT digital signature streams
    println!("\n=== Test 2: Resave without digital signatures ===");
    let streams_no_sig: Vec<ole::OleStream> = streams.iter()
        .filter(|s| !s.name.contains("DigitalSignature") && !s.name.contains("MsiDigitalSignatureEx"))
        .cloned()
        .collect();
    let rebuilt2 = ole::build_ole_file(&streams_no_sig);
    std::fs::write("C:\\Windows\\Temp\\resave_nosig.msi", &rebuilt2).unwrap();
    println!("Written {} bytes ({} streams)", rebuilt2.len(), streams_no_sig.len());
    test_msiexec("C:\\Windows\\Temp\\resave_nosig.msi");
    
    // Test 3: Only small metadata streams (no cabinets, no signatures)
    println!("\n=== Test 3: Only metadata streams (< 100KB) ===");
    let streams_meta: Vec<ole::OleStream> = streams.iter()
        .filter(|s| s.data.len() < 100_000 && !s.name.contains("DigitalSignature") && !s.name.contains("MsiDigitalSignatureEx"))
        .cloned()
        .collect();
    let rebuilt3 = ole::build_ole_file(&streams_meta);
    std::fs::write("C:\\Windows\\Temp\\resave_meta.msi", &rebuilt3).unwrap();
    println!("Written {} bytes ({} streams)", rebuilt3.len(), streams_meta.len());
    test_msiexec("C:\\Windows\\Temp\\resave_meta.msi");
    
    // Test 4: Only the essential system streams (_Tables, _Columns, _Validation, _StringPool, _StringData, SummaryInformation)
    println!("\n=== Test 4: Only essential system streams ===");
    let essential_names: Vec<&str> = vec![
        "\u{0005}SummaryInformation",
    ];
    let streams_essential: Vec<ole::OleStream> = streams.iter()
        .filter(|s| {
            essential_names.iter().any(|&n| s.name == n) ||
            // Keep all TABLE_PREFIX streams (system tables + string pool)
            (s.name.encode_utf16().next() == Some(0x4840) && s.data.len() < 100_000)
        })
        .cloned()
        .collect();
    let rebuilt4 = ole::build_ole_file(&streams_essential);
    std::fs::write("C:\\Windows\\Temp\\resave_essential.msi", &rebuilt4).unwrap();
    println!("Written {} bytes ({} streams)", rebuilt4.len(), streams_essential.len());
    test_msiexec("C:\\Windows\\Temp\\resave_essential.msi");
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
        } else {
            println!("  Result: code={}", code);
        }
    } else {
        println!("  Result: code={} (no log)", code);
    }
}
