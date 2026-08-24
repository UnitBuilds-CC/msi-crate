/// Test: Open COM-created MSI with msi crate, add SummaryInfo, test with msiexec
/// This combines COM's correct database structure with msi crate's SummaryInfo
///
/// cargo run --example com_msi_hybrid -p velocity-msi
use std::io::Cursor;

fn main() {
    println!("=== COM + MSI CRATE HYBRID TEST ===\n");

    let base_path = "C:\\temp\\com_base.msi";
    let out_path = "C:\\temp\\com_hybrid.msi";

    if !std::path::Path::new(base_path).exists() {
        println!("Base MSI not found: {}", base_path);
        println!("Run: cscript //nologo scripts\\create_com_base.vbs");
        return;
    }

    // Kill any running msiexec
    let _ = std::process::Command::new("taskkill")
        .args(&["/F", "/IM", "msiexec.exe"]).output();
    std::thread::sleep(std::time::Duration::from_secs(1));

    // Step 1: Open COM-created MSI with msi crate
    let data = std::fs::read(base_path).unwrap();
    println!("COM base: {} bytes", data.len());

    let cursor = Cursor::new(data);
    let mut pkg = match msi::Package::open(cursor) {
        Ok(p) => { println!("Opened COM MSI OK"); p }
        Err(e) => { println!("Open failed: {:?}", e); return; }
    };

    // Step 2: Read existing tables
    let tables: Vec<String> = pkg.tables().map(|t| t.name().to_string()).collect();
    println!("Existing tables: {:?}", tables);

    // Step 3: Set SummaryInfo using msi crate
    {
        let si = pkg.summary_info_mut();
        si.set_title("Velocity Test");
        si.set_codepage(msi::CodePage::Windows1252);
        si.set_arch("x86");
        si.set_languages(&[msi::Language::from_code(1033)]);
        si.set_uuid(uuid::Uuid::parse_str("AABBCCDD-1234-4567-89AB-CDEF01234567").unwrap());
        si.set_word_count(2);
        si.set_creating_application("Velocity Installer");
    }
    println!("SummaryInfo set");

    // Step 4: Flush
    match pkg.flush() {
        Ok(_) => println!("Flush OK"),
        Err(e) => { println!("Flush failed: {:?}", e); return; }
    }

    // Get the data
    let cursor = pkg.into_inner().unwrap();
    let msi_data = cursor.into_inner();
    std::fs::write(out_path, &msi_data).unwrap();
    println!("Wrote: {} ({} bytes)", out_path, msi_data.len());

    // Step 5: Test with msiexec
    println!("\n--- Testing with msiexec ---");
    let _ = std::fs::remove_file("C:\\temp\\com_hybrid.log");
    let output = std::process::Command::new("msiexec")
        .args(&["/i", out_path, "/qn", "/l*v", "C:\\temp\\com_hybrid.log"])
        .output().unwrap();
    let exit_code = output.status.code().unwrap_or(-1);
    println!("Exit code: {}", exit_code);
    match exit_code {
        0 => println!("SUCCESS!"),
        1603 => println!("1603 (fatal error - but MSI was OPENED!)"),
        1613 => println!("1613 (invalid package)"),
        1619 => println!("1619 (not valid)"),
        1620 => println!("1620 (could not open)"),
        _ => println!("Error {}", exit_code),
    }

    // Check log
    if let Ok(log) = std::fs::read_to_string("C:\\temp\\com_hybrid.log") {
        println!("\nLog highlights:");
        for line in log.lines() {
            if line.contains("Error") || line.contains("2219") ||
               line.contains("return value 3") || line.contains("Product:") {
                println!("  {}", line.trim());
            }
        }
    }

    println!("\n=== DONE ===");
}
