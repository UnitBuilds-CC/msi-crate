/// Test: repackage a known-good MSI through cfb to verify OLE container validity.
/// If the repackaged MSI works → cfb is fine, issue is in MSI data.
/// If it doesn't → cfb is producing invalid OLE for MSI.
///
/// cargo run --example cfb_roundtrip_test -p velocity-msi
use std::io::{Cursor, Read, Write};

fn main() {
    println!("=== CFB ROUNDTRIP TEST ===\n");

    let _ = std::process::Command::new("taskkill")
        .args(&["/F", "/IM", "msiexec.exe"]).output();
    std::thread::sleep(std::time::Duration::from_secs(2));

    let template_path = "C:\\Program Files\\Microsoft Office\\root\\Integration\\C2RIntLoc.en-us.16.msi";
    let template_data = std::fs::read(template_path).unwrap();
    println!("Template MSI: {} bytes", template_data.len());

    // Open with cfb and read all streams
    let mut comp = cfb::CompoundFile::open(Cursor::new(&template_data)).unwrap();
    let mut streams: Vec<(String, Vec<u8>)> = Vec::new();
    let names: Vec<String> = comp.walk()
        .filter(|e| e.is_stream())
        .map(|e| e.name().to_string())
        .collect();
    println!("Streams: {}", names.len());
    for name in &names {
        let mut stream = comp.open_stream(name).unwrap();
        let mut data = Vec::new();
        stream.read_to_end(&mut data).unwrap();
        let safe: String = name.chars().map(|c| if c.is_ascii() && !c.is_control() { c } else { '.' }).collect();
        println!("  {} ({} bytes)", safe, data.len());
        streams.push((name.clone(), data));
    }

    // Create a new OLE file with cfb, copying all streams
    let out_path = "C:\\temp\\cfb_roundtrip.msi";
    {
        let mut buf = Vec::new();
        {
            let cursor = Cursor::new(&mut buf);
            let mut new_comp = cfb::CompoundFile::create_with_version(
                cfb::Version::V3, cursor,
            ).unwrap();

            // Set MSI CLSID on root entry
            let msi_clsid = uuid::Uuid::from_bytes([
                0x00, 0x0C, 0x10, 0x84, 0x00, 0x00, 0x00, 0x00,
                0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46,
            ]);
            new_comp.set_storage_clsid("", msi_clsid).unwrap();

            // Copy all streams
            for (name, data) in &streams {
                let mut s = new_comp.create_stream(name).unwrap();
                s.write_all(data).unwrap();
            }
            new_comp.flush().unwrap();
        }
        std::fs::write(out_path, &buf).unwrap();
        println!("\nRoundtrip MSI: {} bytes", buf.len());
    }

    // Test with msiexec
    println!("\n--- Testing roundtrip MSI with msiexec ---");
    let output = std::process::Command::new("msiexec")
        .args(&["/i", out_path, "/qn", "/l*v", "C:\\temp\\cfb_roundtrip.log"])
        .output().unwrap();
    let exit_code = output.status.code().unwrap_or(-1);
    println!("Exit code: {}", exit_code);
    match exit_code {
        0 => println!("SUCCESS! Roundtrip MSI opened/installed!"),
        1603 => println!("1603 (fatal error - but package was ACCEPTED)"),
        1613 => println!("1613 (invalid package - OLE/CLSID issue)"),
        1619 => println!("1619 (invalid installation package)"),
        1620 => println!("1620 (could not open - OLE/CLSID issue)"),
        _ => println!("Error code {}", exit_code),
    }

    // Check log
    if let Ok(log) = std::fs::read_to_string("C:\\temp\\cfb_roundtrip.log") {
        let lines: Vec<&str> = log.lines().collect();
        println!("\n--- Log highlights ---");
        for line in &lines {
            if line.contains("Error") || line.contains("return value") ||
               line.contains("Product:") || line.contains("successful") {
                println!("  {}", line);
            }
        }
    }

    // Also test the original template
    println!("\n--- Testing original template with msiexec ---");
    let output2 = std::process::Command::new("msiexec")
        .args(&["/i", template_path, "/qn", "/l*v", "C:\\temp\\cfb_template.log"])
        .output().unwrap();
    let exit2 = output2.status.code().unwrap_or(-1);
    println!("Original template exit code: {}", exit2);

    println!("\n=== DONE ===");
}
