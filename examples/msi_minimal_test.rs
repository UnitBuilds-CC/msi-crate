/// Minimal test: create MSI from scratch using msi crate only
/// cargo run --example msi_minimal_test -p velocity-msi
use std::io::Cursor;

fn make_uuid() -> uuid::Uuid {
    // Simple pseudo-random UUID v4
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let a = (t & 0xFFFFFFFF) as u32;
    let b = ((t >> 32) & 0xFFFF) as u16;
    let c = (((t >> 48) & 0x0FFF) as u16) | 0x4000; // version 4
    let d = [0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01]; // variant 1
    uuid::Uuid::from_fields(a, b, c, &d)
}

fn main() {
    println!("=== MINIMAL MSI CRATE TEST ===\n");

    let out_path = "C:\\temp\\msi_minimal.msi";
    let _ = std::fs::remove_file(out_path);
    let _ = std::fs::remove_file("C:\\temp\\msi_minimal.log");

    // Create a new MSI package from scratch
    let cursor = Cursor::new(Vec::new());
    let mut pkg = match msi::Package::create(msi::PackageType::Installer, cursor) {
        Ok(p) => { println!("Package created"); p }
        Err(e) => { println!("Create failed: {:?}", e); return; }
    };

    // Set SummaryInfo
    {
        let si = pkg.summary_info_mut();
        si.set_title("Velocity Test Installation");
        si.set_subject("Velocity Test");
        si.set_author("Velocity Corp");
        si.set_codepage(msi::CodePage::Windows1252);
        si.set_arch("Intel");
        si.set_languages(&[msi::Language::from_code(1033)]);
        si.set_word_count(2);
        si.set_uuid(make_uuid());
        si.set_creating_application("Velocity Installer");
        println!("SummaryInfo set");
    }

    // Set database codepage
    pkg.set_database_codepage(msi::CodePage::Windows1252);

    // Create Property table
    {
        let columns = vec![
            msi::Column::build("Property").primary_key().string(72),
            msi::Column::build("Value").nullable().localizable().string(255),
        ];
        match pkg.create_table("Property", columns) {
            Ok(_) => println!("Property table created"),
            Err(e) => { println!("create_table failed: {:?}", e); return; }
        }
    }

    // Insert Property rows using the Insert query API
    let product_code = format!("{{{}}}", make_uuid().hyphenated().to_string().to_uppercase());
    let upgrade_code = format!("{{{}}}", make_uuid().hyphenated().to_string().to_uppercase());

    let insert = msi::Insert::into("Property")
        .row(vec![msi::Value::Str("ProductName".into()), msi::Value::Str("Velocity Test".into())])
        .row(vec![msi::Value::Str("ProductVersion".into()), msi::Value::Str("1.0.0".into())])
        .row(vec![msi::Value::Str("Manufacturer".into()), msi::Value::Str("Velocity Corp".into())])
        .row(vec![msi::Value::Str("ProductCode".into()), msi::Value::Str(product_code.clone())])
        .row(vec![msi::Value::Str("UpgradeCode".into()), msi::Value::Str(upgrade_code.clone())])
        .row(vec![msi::Value::Str("ProductLanguage".into()), msi::Value::Str("1033".into())]);

    match pkg.insert_rows(insert) {
        Ok(_) => println!("Property rows inserted"),
        Err(e) => { println!("Insert failed: {:?}", e); return; }
    }

    // Flush to disk
    match pkg.flush() {
        Ok(_) => println!("Flush OK"),
        Err(e) => { println!("Flush failed: {:?}", e); return; }
    }

    // Write to file
    let cursor = pkg.into_inner().unwrap();
    let msi_data = cursor.into_inner();
    std::fs::write(out_path, &msi_data).unwrap();
    println!("Wrote: {} ({} bytes)", out_path, msi_data.len());

    // Test with msiexec
    println!("\n--- msiexec test ---");
    let output = std::process::Command::new("msiexec")
        .args(&["/i", out_path, "/qn", "/l*v", "C:\\temp\\msi_minimal.log"])
        .output().unwrap();
    let exit_code = output.status.code().unwrap_or(-1);
    println!("Exit code: {}", exit_code);
    match exit_code {
        0 => println!("SUCCESS!"),
        1603 => println!("1603 (accepted but install error)"),
        1613 => println!("1613 (invalid package)"),
        1619 => println!("1619 (not valid)"),
        1620 => println!("1620 (could not open)"),
        _ => println!("Error {}", exit_code),
    }

    if let Ok(log) = std::fs::read_to_string("C:\\temp\\msi_minimal.log") {
        for line in log.lines() {
            if line.contains("Error") || line.contains("2219") || line.contains("2203") ||
               line.contains("return value 3") || line.contains("Product:") ||
               line.contains("Could not") || line.contains("Installation successful") {
                println!("  {}", line.trim());
            }
        }
    }

    println!("\n=== DONE ===");
}
