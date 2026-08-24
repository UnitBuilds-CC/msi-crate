/// Test: COM database + velocity-msi SummaryInfo + cfb OLE
/// Then open with msi crate and re-flush for compatibility
/// cargo run --example full_hybrid_test -p velocity-msi
use std::io::{Cursor, Read, Write};

fn main() {
    println!("=== FULL HYBRID TEST ===\n");

    let base_path = "C:\\temp\\com_base.msi";
    let out_path = "C:\\temp\\full_hybrid.msi";

    if !std::path::Path::new(base_path).exists() {
        println!("Run: cscript //nologo scripts\\create_com_base.vbs");
        return;
    }

    let _ = std::process::Command::new("taskkill")
        .args(&["/F", "/IM", "msiexec.exe"]).output();
    std::thread::sleep(std::time::Duration::from_secs(1));

    // Step 1: Read COM-created MSI
    let com_data = std::fs::read(base_path).unwrap();
    println!("COM base: {} bytes", com_data.len());

    // Step 2: Read all streams from COM MSI
    let mut streams: Vec<(String, Vec<u8>)> = Vec::new();
    {
        let mut comp = cfb::CompoundFile::open(Cursor::new(&com_data)).unwrap();
        let entries: Vec<(String, bool)> = comp.walk()
            .map(|e| (e.path().to_string_lossy().to_string(), e.is_stream()))
            .collect();
        for (path, is_stream) in &entries {
            if *is_stream {
                let mut data = Vec::new();
                let mut s = comp.open_stream(path).unwrap();
                s.read_to_end(&mut data).unwrap();
                let name = path.rsplit('/').next().unwrap_or(path).to_string();
                println!("  Stream: {} ({} bytes)", 
                    name.chars().map(|c| if c.is_ascii() && !c.is_control() { c } else { '.' }).collect::<String>(),
                    data.len());
                streams.push((name, data));
            }
        }
    }

    // Step 3: Build SummaryInfo using velocity-msi
    let summary_data = {
        let mut si = velocity_msi::SummaryInfo::new();
        si.title = Some("Velocity Test".to_string());
        si.template = Some("x86;1033".to_string());
        si.rev_number = Some("{AABBCCDD-1234-4567-89AB-CDEF01234567}".to_string());
        si.creating_app = Some("Velocity Installer".to_string());
        si.codepage = 1252;
        si.word_count = 2;
        si.security = 405;
        si.serialize().unwrap()
    };
    println!("SummaryInfo: {} bytes", summary_data.len());

    // Step 4: Create fresh V3 CFB with all streams
    let mut buf = Vec::new();
    {
        let cursor = Cursor::new(&mut buf);
        let mut comp = cfb::CompoundFile::create_with_version(
            cfb::Version::V3, cursor,
        ).unwrap();

        // Set MSI CLSID
        let msi_clsid = uuid::Uuid::from_bytes([
            0x00, 0x0C, 0x10, 0x84, 0x00, 0x00, 0x00, 0x00,
            0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46,
        ]);
        comp.set_storage_clsid("", msi_clsid).unwrap();

        // Copy all COM streams
        for (name, data) in &streams {
            let mut s = comp.create_stream(name).unwrap();
            s.write_all(data).unwrap();
        }

        // Add SummaryInfo
        let mut s = comp.create_stream("\u{0005}SummaryInformation").unwrap();
        s.write_all(&summary_data).unwrap();

        comp.flush().unwrap();
    }
    std::fs::write(out_path, &buf).unwrap();
    println!("\nWrote: {} ({} bytes)", out_path, buf.len());

    // Step 5: Test with msiexec
    println!("\n--- msiexec test ---");
    let _ = std::fs::remove_file("C:\\temp\\full_hybrid.log");
    let output = std::process::Command::new("msiexec")
        .args(&["/i", out_path, "/qn", "/l*v", "C:\\temp\\full_hybrid.log"])
        .output().unwrap();
    let exit_code = output.status.code().unwrap_or(-1);
    println!("Exit code: {}", exit_code);
    match exit_code {
        0 => println!("SUCCESS!"),
        1603 => println!("1603 (accepted but install failed)"),
        1613 => println!("1613 (invalid package)"),
        1619 => println!("1619 (not valid)"),
        1620 => println!("1620 (could not open)"),
        1633 => println!("1633 (not supported by interface)"),
        _ => println!("Error {}", exit_code),
    }

    if let Ok(log) = std::fs::read_to_string("C:\\temp\\full_hybrid.log") {
        println!("\nLog highlights:");
        for line in log.lines() {
            if line.contains("Error") || line.contains("2219") ||
               line.contains("return value 3") || line.contains("Product:") ||
               line.contains("Installation successful") || line.contains("Could not") {
                println!("  {}", line.trim());
            }
        }
    }

    // Step 6: Also test opening with msi crate
    println!("\n--- msi crate test ---");
    let data2 = std::fs::read(out_path).unwrap();
    let cursor2 = Cursor::new(data2);
    match msi::Package::open(cursor2) {
        Ok(pkg) => {
            println!("msi crate opened OK!");
            let tables: Vec<String> = pkg.tables().map(|t| t.name().to_string()).collect();
            println!("  Tables: {:?}", tables);
        }
        Err(e) => println!("msi crate open failed: {:?}", e),
    }

    println!("\n=== DONE ===");
}
