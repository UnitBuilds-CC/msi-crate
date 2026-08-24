/// Test: Create a minimal MSI from scratch using the msi crate's create() path.
/// Even though create() was reported broken (1620), let's test it with a minimal
/// MSI to see if it's specific to complex MSIs or all create() output.
///
/// cargo run --example msi_create_test -p velocity-msi
use std::io::Cursor;

fn main() {
    println!("=== MSI CRATE CREATE TEST ===\n");

    let _ = std::process::Command::new("taskkill")
        .args(&["/F", "/IM", "msiexec.exe"]).output();
    std::thread::sleep(std::time::Duration::from_secs(1));

    let out_path = "C:\\temp\\msi_crate_create.msi";

    // Create a minimal MSI using the msi crate
    let mut buf = Vec::new();
    {
        let cursor = Cursor::new(&mut buf);
        let mut pkg = match msi::Package::create(msi::PackageType::Installer, cursor) {
            Ok(p) => { println!("Created package OK"); p }
            Err(e) => { println!("Create failed: {:?}", e); return; }
        };

        // Create Property table
        println!("Creating Property table...");
        match pkg.create_table("Property", vec![
            msi::Column::build("Property").primary_key().string(72),
            msi::Column::build("Value").localizable().nullable().string(0),
        ]) {
            Ok(_) => println!("  Table created"),
            Err(e) => { println!("  Create table failed: {:?}", e); return; }
        }

        // Insert rows
        let product_code = format!("{{{:08X}-{:04X}-4{:03X}-8{:03X}-{:012X}}}",
            0xAABBCCDDu32, 0x1234u16, 0x567u16, 0x89Au16, 0xBCDEF012345u64);
        let upgrade_code = format!("{{{:08X}-{:04X}-4{:03X}-8{:03X}-{:012X}}}",
            0x11223344u32, 0x5678u16, 0x9ABu16, 0xCDEu16, 0xF0123456789Au64);

        println!("Inserting rows...");
        let rows = vec![
            vec![msi::Value::Str("ProductName".into()), msi::Value::Str("Velocity Test".into())],
            vec![msi::Value::Str("ProductVersion".into()), msi::Value::Str("1.0.0".into())],
            vec![msi::Value::Str("Manufacturer".into()), msi::Value::Str("Velocity Corp".into())],
            vec![msi::Value::Str("ProductCode".into()), msi::Value::Str(product_code.into())],
            vec![msi::Value::Str("UpgradeCode".into()), msi::Value::Str(upgrade_code.into())],
            vec![msi::Value::Str("ProductLanguage".into()), msi::Value::Str("1033".into())],
        ];
        match pkg.insert_rows(msi::Insert::into("Property").rows(rows)) {
            Ok(_) => println!("  Rows inserted"),
            Err(e) => { println!("  Insert failed: {:?}", e); return; }
        }

        // Set SummaryInfo
        {
            let si = pkg.summary_info_mut();
            si.set_title("Velocity Test Installer");
            si.set_codepage(msi::CodePage::Windows1252);
            si.set_arch("x86");
            si.set_languages(&[msi::Language::from_code(1033)]);
            si.set_uuid(uuid::Uuid::from_fields(0xAABBCCDD, 0x1234, 0x4567, &[0x89, 0xAB, 0xCD, 0xEF, 0x01, 0x23, 0x45, 0x67]));
            si.set_word_count(2);
            si.set_creating_application("Velocity Installer");
        }

        // Flush
        println!("Flushing...");
        match pkg.flush() {
            Ok(_) => println!("Flush OK"),
            Err(e) => { println!("Flush failed: {:?}", e); return; }
        }
    }

    std::fs::write(out_path, &buf).unwrap();
    println!("Wrote: {} ({} bytes)", out_path, buf.len());

    // Test with msiexec
    println!("\n--- Testing with msiexec ---");
    let _ = std::fs::remove_file("C:\\temp\\msi_create.log");
    let output = std::process::Command::new("msiexec")
        .args(&["/i", out_path, "/qn", "/l*v", "C:\\temp\\msi_create.log"])
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
    if let Ok(log) = std::fs::read_to_string("C:\\temp\\msi_create.log") {
        println!("\nLog highlights:");
        for line in log.lines() {
            if line.contains("Error") || line.contains("return value 3") ||
               line.contains("Product:") || line.contains("Could not open") {
                println!("  {}", line.trim());
            }
        }
    }

    println!("\n=== DONE ===");
}
