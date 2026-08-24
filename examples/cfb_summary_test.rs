/// Test: Add SummaryInfo to COM-created MSI via cfb, then open with msi crate
/// cargo run --example cfb_summary_test -p velocity-msi
use std::io::{Cursor, Write};

fn main() {
    println!("=== CFB SUMMARY TEST ===\n");

    let base_path = "C:\\temp\\com_base.msi";
    let out_path = "C:\\temp\\cfb_summary.msi";

    if !std::path::Path::new(base_path).exists() {
        println!("Base not found. Run: cscript //nologo scripts\\create_com_base.vbs");
        return;
    }

    // Kill msiexec
    let _ = std::process::Command::new("taskkill")
        .args(&["/F", "/IM", "msiexec.exe"]).output();
    std::thread::sleep(std::time::Duration::from_secs(1));

    // Step 1: Open COM MSI with cfb and add SummaryInfo stream
    let data = std::fs::read(base_path).unwrap();
    println!("COM base: {} bytes", data.len());

    // Build a minimal SummaryInfo stream using velocity-msi's serializer
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
    println!("SummaryInfo stream: {} bytes", summary_data.len());

    // Open as CFB and add SummaryInfo
    let mut comp = cfb::CompoundFile::open(Cursor::new(data)).unwrap();
    {
        let mut stream = comp.create_stream("\u{0005}SummaryInformation").unwrap();
        stream.write_all(&summary_data).unwrap();
    }

    // Set MSI CLSID
    let msi_clsid = uuid::Uuid::from_bytes([
        0x00, 0x0C, 0x10, 0x84, 0x00, 0x00, 0x00, 0x00,
        0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46,
    ]);
    comp.set_storage_clsid("", msi_clsid).unwrap();

    comp.flush().unwrap();

    // Get the inner data
    let cursor = comp.into_inner();
    let msi_data = cursor.into_inner();
    std::fs::write(out_path, &msi_data).unwrap();
    println!("Wrote: {} ({} bytes)", out_path, msi_data.len());

    // Step 2: Test with msiexec
    println!("\n--- Testing with msiexec ---");
    let _ = std::fs::remove_file("C:\\temp\\cfb_summary.log");
    let output = std::process::Command::new("msiexec")
        .args(&["/i", out_path, "/qn", "/l*v", "C:\\temp\\cfb_summary.log"])
        .output().unwrap();
    let exit_code = output.status.code().unwrap_or(-1);
    println!("Exit code: {}", exit_code);
    match exit_code {
        0 => println!("SUCCESS!"),
        1603 => println!("1603 (fatal error during install - but MSI opened!)"),
        1613 => println!("1613 (invalid package)"),
        1619 => println!("1619 (not valid)"),
        1620 => println!("1620 (could not open)"),
        _ => println!("Error {}", exit_code),
    }

    // Check log
    if let Ok(log) = std::fs::read_to_string("C:\\temp\\cfb_summary.log") {
        println!("\nLog highlights:");
        for line in log.lines() {
            if line.contains("Error") || line.contains("2219") ||
               line.contains("return value 3") || line.contains("Product:") ||
               line.contains("Installation successful") {
                println!("  {}", line.trim());
            }
        }
    }

    // Step 3: Also try opening with msi crate
    println!("\n--- Opening with msi crate ---");
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
