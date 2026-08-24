/// Test: Open a COM-created MSI with the msi crate, modify SummaryInfo, save.
/// cargo run --example msi_open_com_test -p velocity-msi
use std::io::Cursor;

fn main() {
    println!("=== OPEN COM MSI WITH MSI CRATE ===\n");

    let _ = std::process::Command::new("taskkill")
        .args(&["/F", "/IM", "msiexec.exe"]).output();
    std::thread::sleep(std::time::Duration::from_secs(1));

    let com_msi = "C:\\Program Files\\Microsoft Office\\root\\Integration\\C2RIntLoc.en-us.16.msi";
    let out_msi = "C:\\temp\\msi_crate_modified.msi";

    // Read the template MSI
    let data = std::fs::read(com_msi).unwrap();
    println!("Template MSI: {} bytes", data.len());

    // Open with msi crate (in-memory)
    let pkg = match msi::Package::open(Cursor::new(&data)) {
        Ok(p) => { println!("Opened OK!"); p }
        Err(e) => { println!("Failed: {:?}", e); return; }
    };

    // List tables
    println!("\nTables:");
    for table in pkg.tables() {
        println!("  {} ({} cols)", table.name(), table.columns().len());
        for col in table.columns() {
            println!("    {} ({}) nullable={} pk={}",
                col.name(), col.coltype(), col.is_nullable(), col.is_primary_key());
        }
    }

    // Check SummaryInfo
    let si = pkg.summary_info();
    println!("\nSummaryInfo:");
    println!("  Title: {:?}", si.title());
    println!("  Codepage: {:?}", si.codepage());
    println!("  UUID: {:?}", si.uuid());
    println!("  Word count: {:?}", si.word_count());
    println!("  Creating app: {:?}", si.creating_application());

    drop(pkg);

    // Now try the file-based approach: copy, open for editing, modify, flush
    println!("\n--- File-based approach ---");
    std::fs::copy(com_msi, out_msi).unwrap();

    let file = std::fs::OpenOptions::new()
        .read(true).write(true)
        .open(out_msi).unwrap();

    let mut pkg2 = match msi::Package::open(file) {
        Ok(p) => { println!("Opened copy for editing!"); p }
        Err(e) => { println!("Failed: {:?}", e); return; }
    };

    // Modify SummaryInfo
    {
        let si = pkg2.summary_info_mut();
        si.set_title("Velocity Test Installer");
        si.set_codepage(msi::CodePage::Windows1252);
        si.set_arch("x86");
        si.set_languages(&[msi::Language::from_code(1033)]);
        si.set_uuid(uuid::Uuid::from_fields(0x12345678, 0xABCD, 0x4BBB, &[0x8B, 0xBB, 0xEE, 0xEE, 0xEE, 0xEE, 0xEE, 0xEE]));
        si.set_word_count(2);
        si.set_creating_application("Velocity Installer");
    }

    println!("Flushing...");
    match pkg2.flush() {
        Ok(_) => println!("Flush OK!"),
        Err(e) => { println!("Flush failed: {:?}", e); return; }
    }
    drop(pkg2);

    let new_size = std::fs::metadata(out_msi).unwrap().len();
    println!("Output: {} bytes", new_size);

    // Test with msiexec
    println!("\n--- Testing with msiexec ---");
    let _ = std::fs::remove_file("C:\\temp\\msi_modified.log");
    let output = std::process::Command::new("msiexec")
        .args(&["/i", out_msi, "/qn", "/l*v", "C:\\temp\\msi_modified.log"])
        .output().unwrap();
    let exit_code = output.status.code().unwrap_or(-1);
    println!("Exit code: {}", exit_code);
    match exit_code {
        0 => println!("SUCCESS!"),
        1603 => println!("1603 (accepted, install failed - needs cabinet)"),
        1619 => println!("1619 (invalid installation package)"),
        1620 => println!("1620 (could not open)"),
        _ => println!("Error {}", exit_code),
    }

    // Check log
    if let Ok(log) = std::fs::read_to_string("C:\\temp\\msi_modified.log") {
        println!("\nLog highlights:");
        for line in log.lines() {
            if line.contains("Error") || line.contains("return value 3") ||
               line.contains("Product:") || line.contains("Could not open") ||
               line.contains("MSI") {
                println!("  {}", line.trim());
            }
        }
    }

    println!("\n=== DONE ===");
}
