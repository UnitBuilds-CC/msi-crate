/// Definitive diagnostic: Use cfb crate to write the same streams
/// and test if msiexec accepts the result.
///
/// If cfb-written file works → issue is in our OLE writer
/// If cfb-written file fails → issue is in content/streams
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
    
    let mut streams: Vec<(String, Vec<u8>)> = Vec::new();
    for (p, is_stream) in entries {
        if !is_stream { continue; }
        let name = p.file_name().unwrap().to_string_lossy().to_string();
        let mut stream = comp.open_stream(&p).unwrap();
        let mut data = Vec::new();
        stream.read_to_end(&mut data).unwrap();
        streams.push((name, data));
    }
    println!("Read {} streams from system MSI", streams.len());
    
    // Test 1: Write exact same streams using cfb crate (in-memory then to disk)
    println!("\n=== Test 1: cfb roundtrip (all streams) ===");
    {
        let cursor = Cursor::new(Vec::new());
        let mut out = cfb::OpenOptions::new().create_with(cursor).unwrap();
        for (name, data) in &streams {
            let path = format!("/{}", name);
            let mut s = out.create_stream(&path).unwrap();
            s.write_all(data).unwrap();
        }
        let cursor = out.into_inner();
        let cfb_data = cursor.into_inner();
        let out_path = "C:\\Windows\\Temp\\cfb_roundtrip.msi";
        std::fs::write(out_path, &cfb_data).unwrap();
        println!("Written {} bytes via cfb", cfb_data.len());
    }
    test_msiexec("C:\\Windows\\Temp\\cfb_roundtrip.msi");
    
    // Test 2: Write only essential streams via cfb
    println!("\n=== Test 2: cfb roundtrip (essential streams only) ===");
    {
        let cursor = Cursor::new(Vec::new());
        let mut out = cfb::OpenOptions::new().create_with(cursor).unwrap();
        let essential_names: Vec<&str> = vec!["\u{0005}SummaryInformation"];
        for (name, data) in &streams {
            let is_essential = essential_names.iter().any(|&n| name == n)
                || (name.encode_utf16().next() == Some(0x4840) && data.len() < 100_000);
            if !is_essential { continue; }
            
            let path = format!("/{}", name);
            let mut s = out.create_stream(&path).unwrap();
            s.write_all(data).unwrap();
        }
        let cursor = out.into_inner();
        let cfb_data = cursor.into_inner();
        let out_path = "C:\\Windows\\Temp\\cfb_essential.msi";
        std::fs::write(out_path, &cfb_data).unwrap();
        println!("Written {} bytes via cfb (essential only)", cfb_data.len());
    }
    test_msiexec("C:\\Windows\\Temp\\cfb_essential.msi");
    
    // Test 3: Also test our OLE writer output for comparison
    println!("\n=== Test 3: Our OLE writer (all streams) ===");
    {
        let ole_streams: Vec<velocity_msi::ole::OleStream> = streams.iter()
            .map(|(name, data)| velocity_msi::ole::OleStream {
                name: name.clone(),
                data: data.clone(),
            })
            .collect();
        let our_data = velocity_msi::ole::build_ole_file(&ole_streams);
        let our_path = "C:\\Windows\\Temp\\our_ole_resave.msi";
        std::fs::write(our_path, &our_data).unwrap();
        println!("Written {} bytes via our OLE writer", our_data.len());
    }
    test_msiexec("C:\\Windows\\Temp\\our_ole_resave.msi");
    
    // Test 4: Compare file sizes and headers
    println!("\n=== File comparison ===");
    let orig = std::fs::read(sys_path).unwrap();
    let cfb_out = std::fs::read("C:\\Windows\\Temp\\cfb_roundtrip.msi").unwrap();
    let our_out = std::fs::read("C:\\Windows\\Temp\\our_ole_resave.msi").unwrap();
    
    println!("Original:  {} bytes", orig.len());
    println!("cfb:       {} bytes", cfb_out.len());
    println!("Our OLE:   {} bytes", our_out.len());
    
    // Compare headers
    println!("\nHeader diff (orig vs cfb):");
    print_header_diff(&orig, &cfb_out, "cfb");
    println!("\nHeader diff (orig vs our):");
    print_header_diff(&orig, &our_out, "our");
    println!("\nHeader diff (cfb vs our):");
    print_header_diff(&cfb_out, &our_out, "our");
    
    // Also compare using cfb: read our file and verify structure
    println!("\n=== Structural validation ===");
    validate_with_cfb("C:\\Windows\\Temp\\our_ole_resave.msi");
}

fn validate_with_cfb(path: &str) {
    let file = File::open(path).unwrap();
    match cfb::CompoundFile::open(file) {
        Ok(mut comp) => {
            let entries: Vec<_> = comp.walk().collect();
            println!("  cfb can read our file: {} entries", entries.len());
            for e in &entries {
                if e.is_stream() {
                    let p = e.path();
                    match comp.open_stream(p) {
                        Ok(mut s) => {
                            let mut buf = Vec::new();
                            s.read_to_end(&mut buf).unwrap();
                            let name = p.file_name().unwrap().to_string_lossy();
                            println!("    Stream '{}': {} bytes", name, buf.len());
                        }
                        Err(e) => {
                            let name = p.file_name().unwrap().to_string_lossy();
                            println!("    Stream '{}': ERROR opening: {}", name, e);
                        }
                    }
                }
            }
        }
        Err(e) => println!("  cfb CANNOT read our file: {}", e),
    }
}

fn print_header_diff(a: &[u8], b: &[u8], label: &str) {
    let mut diffs = Vec::new();
    let len = a.len().min(b.len()).min(512);
    for i in 0..len {
        if a[i] != b[i] {
            diffs.push((i, a[i], b[i]));
        }
    }
    if diffs.is_empty() {
        println!("  {}: identical header (first {} bytes)", label, len);
    } else {
        println!("  {}: {} byte differences in header:", label, diffs.len());
        for (offset, a_byte, b_byte) in diffs.iter().take(30) {
            // Show what field this offset corresponds to
            let field = match offset {
                0..=7 => "magic",
                8..=23 => "CLSID",
                24..=25 => "minor_ver",
                26..=27 => "major_ver",
                28..=29 => "byte_order",
                30..=31 => "sector_shift",
                32..=33 => "mini_sector_shift",
                34..=39 => "reserved",
                40..=43 => "dir_sectors",
                44..=47 => "fat_sectors",
                48..=51 => "first_dir_sector",
                52..=55 => "transaction_sig",
                56..=59 => "mini_stream_cutoff",
                60..=63 => "first_minifat_sector",
                64..=67 => "minifat_sectors",
                68..=71 => "first_difat",
                72..=75 => "difat_count",
                _ => "difat_array",
            };
            println!("    [{}] offset 0x{:04X}: A=0x{:02X} B=0x{:02X}", field, offset, a_byte, b_byte);
        }
    }
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
            println!("  Result: Error 1603 (fatal error)");
            // Show relevant log lines
            for line in log.lines() {
                if line.contains("error") || line.contains("Error") || line.contains("return value 3") {
                    println!("    LOG: {}", line.trim());
                }
            }
        } else {
            println!("  Result: code={}", code);
        }
    } else {
        println!("  Result: code={} (no log)", code);
    }
}
