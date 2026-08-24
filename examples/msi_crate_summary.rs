/// Test: COM database + cfb SummaryInfo + msi crate flush
/// cargo run --example msi_crate_summary -p velocity-msi
use std::io::{Cursor, Write};

fn main() {
    println!("=== MSI CRATE SUMMARY TEST ===\n");

    let base_path = "C:\\temp\\com_base.msi";
    let out_path = "C:\\temp\\msi_crate_summary.msi";

    if !std::path::Path::new(base_path).exists() {
        println!("Run: cscript //nologo scripts\\create_com_base.vbs");
        return;
    }

    let _ = std::process::Command::new("taskkill")
        .args(&["/F", "/IM", "msiexec.exe"]).output();
    std::thread::sleep(std::time::Duration::from_secs(1));

    // Step 1: Read COM MSI and add minimal SummaryInfo via cfb
    let com_data = std::fs::read(base_path).unwrap();
    println!("COM base: {} bytes", com_data.len());

    let minimal_summary = {
        let mut si = velocity_msi::SummaryInfo::new();
        si.codepage = 1252;
        si.rev_number = Some("{00000000-0000-0000-0000-000000000000}".to_string());
        si.serialize().unwrap()
    };

    // Add SummaryInfo to COM MSI via cfb
    let modified_data = {
        let mut comp = cfb::CompoundFile::open(Cursor::new(com_data)).unwrap();

        // Add SummaryInfo stream
        let mut s = comp.create_stream("\u{0005}SummaryInformation").unwrap();
        s.write_all(&minimal_summary).unwrap();

        // Set MSI CLSID
        let msi_clsid = uuid::Uuid::from_bytes([
            0x00, 0x0C, 0x10, 0x84, 0x00, 0x00, 0x00, 0x00,
            0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46,
        ]);
        let _ = comp.set_storage_clsid("", msi_clsid);

        comp.flush().unwrap();
        let cursor = comp.into_inner();
        cursor.into_inner()
    };
    println!("After adding SummaryInfo: {} bytes", modified_data.len());

    // Step 2: Open with msi crate, set proper SummaryInfo, flush
    {
        let cursor = Cursor::new(modified_data);
        let mut pkg = match msi::Package::open(cursor) {
            Ok(p) => { println!("msi crate opened OK"); p }
            Err(e) => { println!("msi crate open failed: {:?}", e); return; }
        };

        let tables: Vec<String> = pkg.tables().map(|t| t.name().to_string()).collect();
        println!("Tables: {:?}", tables);

        // Set proper SummaryInfo via msi crate
        let si = pkg.summary_info_mut();
        si.set_title("Velocity Test");
        si.set_codepage(msi::CodePage::Windows1252);
        si.set_arch("x86");
        si.set_languages(&[msi::Language::from_code(1033)]);
        si.set_uuid(uuid::Uuid::parse_str("AABBCCDD-1234-4567-89AB-CDEF01234567").unwrap());
        si.set_word_count(2);
        si.set_creating_application("Velocity Installer");
        println!("SummaryInfo set");

        match pkg.flush() {
            Ok(_) => println!("Flush OK"),
            Err(e) => { println!("Flush failed: {:?}", e); return; }
        }

        let cursor = pkg.into_inner().unwrap();
        let msi_data = cursor.into_inner();
        std::fs::write(out_path, &msi_data).unwrap();
        println!("Wrote: {} ({} bytes)", out_path, msi_data.len());
    }

    // Step 3: Test with msiexec
    println!("\n--- msiexec test ---");
    let _ = std::fs::remove_file("C:\\temp\\msi_crate_summary.log");
    let output = std::process::Command::new("msiexec")
        .args(&["/i", out_path, "/qn", "/l*v", "C:\\temp\\msi_crate_summary.log"])
        .output().unwrap();
    let exit_code = output.status.code().unwrap_or(-1);
    println!("Exit code: {}", exit_code);
    match exit_code {
        0 => println!("SUCCESS!"),
        1603 => println!("1603 (accepted but install error)"),
        1613 => println!("1613 (invalid package)"),
        1619 => println!("1619 (not valid)"),
        1620 => println!("1620 (could not open)"),
        1633 => println!("1633 (not supported)"),
        _ => println!("Error {}", exit_code),
    }

    if let Ok(log) = std::fs::read_to_string("C:\\temp\\msi_crate_summary.log") {
        for line in log.lines() {
            if line.contains("Error") || line.contains("2219") ||
               line.contains("return value 3") || line.contains("Product:") ||
               line.contains("Could not") || line.contains("Installation successful") {
                println!("  {}", line.trim());
            }
        }
    }

    println!("\n=== DONE ===");
}
