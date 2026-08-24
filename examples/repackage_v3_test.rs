/// Test: Create MSI with msi crate, then repackage from V4→V3 via cfb.
/// The msi crate's create() uses CFB V4 (4096-byte sectors) but MSI requires
/// V3 (512-byte sectors). This test repackages through cfb to fix the version.
///
/// cargo run --example repackage_v3_test -p velocity-msi
use std::io::{Cursor, Read};

fn main() {
    println!("=== REPACKAGE V4→V3 TEST ===\n");

    let _ = std::process::Command::new("taskkill")
        .args(&["/F", "/IM", "msiexec.exe"]).output();
    std::thread::sleep(std::time::Duration::from_secs(1));

    // Step 1: Create MSI with msi crate (V4)
    let msi_v4 = {
        let mut buf = Vec::new();
        let cursor = Cursor::new(&mut buf);
        let mut pkg = msi::Package::create(msi::PackageType::Installer, cursor).unwrap();

        // Create Property table
        pkg.create_table("Property", vec![
            msi::Column::build("Property").primary_key().string(72),
            msi::Column::build("Value").localizable().nullable().string(0),
        ]).unwrap();

        // Insert rows
        let product_code = "{AABBCCDD-1234-4567-89AB-CDEF01234567}";
        let upgrade_code = "{11223344-5678-49AB-BCDE-F0123456789A}";
        let rows = vec![
            vec![msi::Value::Str("ProductName".into()), msi::Value::Str("Velocity Test".into())],
            vec![msi::Value::Str("ProductVersion".into()), msi::Value::Str("1.0.0".into())],
            vec![msi::Value::Str("Manufacturer".into()), msi::Value::Str("Velocity Corp".into())],
            vec![msi::Value::Str("ProductCode".into()), msi::Value::Str(product_code.into())],
            vec![msi::Value::Str("UpgradeCode".into()), msi::Value::Str(upgrade_code.into())],
            vec![msi::Value::Str("ProductLanguage".into()), msi::Value::Str("1033".into())],
        ];
        pkg.insert_rows(msi::Insert::into("Property").rows(rows)).unwrap();

        // Set SummaryInfo
        {
            let si = pkg.summary_info_mut();
            si.set_title("Velocity Test Installer");
            si.set_codepage(msi::CodePage::Windows1252);
            si.set_arch("x86");
            si.set_languages(&[msi::Language::from_code(1033)]);
            si.set_uuid(uuid::Uuid::from_fields(0xAABBCCDD, 0x1234, 0x4567,
                &[0x89, 0xAB, 0xCD, 0xEF, 0x01, 0x23, 0x45, 0x67]));
            si.set_word_count(2);
            si.set_creating_application("Velocity Installer");
        }
        pkg.flush().unwrap();
        buf
    };
    println!("msi crate V4: {} bytes", msi_v4.len());
    println!("  Version byte: {}", msi_v4[26]);  // Should be 4 for V4

    // Step 2: Read all streams from V4
    let mut streams: Vec<(String, Vec<u8>)> = Vec::new();
    {
        let mut comp = cfb::CompoundFile::open(Cursor::new(&msi_v4)).unwrap();
        let names: Vec<String> = comp.walk()
            .filter(|e| e.is_stream())
            .map(|e| e.name().to_string())
            .collect();
        println!("  Streams: {}", names.len());
        for name in &names {
            let mut stream = comp.open_stream(name).unwrap();
            let mut data = Vec::new();
            stream.read_to_end(&mut data).unwrap();
            let safe: String = name.chars().map(|c| if c.is_ascii() && !c.is_control() { c } else { '.' }).collect();
            println!("    {} ({} bytes)", safe, data.len());
            streams.push((name.clone(), data));
        }
    }

    // Step 3: Create V3 compound file with all streams
    let msi_v3 = {
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

            // Copy all streams
            for (name, data) in &streams {
                let mut s = comp.create_stream(name).unwrap();
                s.write_all(data).unwrap();
            }
            comp.flush().unwrap();
        }
        buf
    };
    println!("\nRepackaged V3: {} bytes", msi_v3.len());
    println!("  Version byte: {}", msi_v3[26]);  // Should be 3 for V3

    // Step 4: Write and test
    let out_path = "C:\\temp\\repackage_v3.msi";
    std::fs::write(out_path, &msi_v3).unwrap();

    println!("\n--- Testing with msiexec ---");
    let _ = std::fs::remove_file("C:\\temp\\repackage_v3.log");
    let output = std::process::Command::new("msiexec")
        .args(&["/i", out_path, "/qn", "/l*v", "C:\\temp\\repackage_v3.log"])
        .output().unwrap();
    let exit_code = output.status.code().unwrap_or(-1);
    println!("Exit code: {}", exit_code);
    match exit_code {
        0 => println!("SUCCESS!"),
        1603 => println!("1603 (accepted, install failed - needs cabinet)"),
        1613 => println!("1613 (invalid package)"),
        1619 => println!("1619 (invalid installation package)"),
        1620 => println!("1620 (could not open)"),
        _ => println!("Error {}", exit_code),
    }

    // Check log
    if let Ok(log) = std::fs::read_to_string("C:\\temp\\repackage_v3.log") {
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
