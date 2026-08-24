/// Test: Compare custom OLE writer vs cfb crate output
use velocity_msi::{MsiBuilder, Column, Value};

fn main() {
    // Build a minimal MSI with velocity-msi (uses custom OLE writer)
    let mut builder = MsiBuilder::new();
    builder.set_title("Test");
    builder.set_author("Test");
    builder.set_template("Intel", 1033);

    builder.create_table("Property", vec![
        Column::build("Property").string(72).primary_key().build(),
        Column::build("Value").string(255).nullable().build(),
    ]).unwrap();
    builder.insert_rows("Property", vec![
        vec![Value::from("ProductName"), Value::from("Test")],
        vec![Value::from("ProductVersion"), Value::from("1.0.0")],
        vec![Value::from("Manufacturer"), Value::from("Test")],
        vec![Value::from("ProductCode"), Value::from("{AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE}")],
        vec![Value::from("UpgradeCode"), Value::from("{11111111-2222-3333-4444-555555555555}")],
        vec![Value::from("ProductLanguage"), Value::from("1033")],
    ]).unwrap();

    let msi_data = builder.build().unwrap();
    println!("Custom OLE writer: {} bytes", msi_data.len());

    // Write to file
    let out_path = "C:\\temp\\test_custom_ole.msi";
    std::fs::write(out_path, &msi_data).unwrap();
    println!("Wrote to {}", out_path);

    // Test with msiexec
    let status = std::process::Command::new("msiexec")
        .args(&["/i", out_path, "/qn", "/norestart"])
        .status()
        .expect("Failed to run msiexec");
    println!("msiexec exit code: {}", status.code().unwrap_or(-1));

    // Uninstall
    let status2 = std::process::Command::new("msiexec")
        .args(&["/x", "{AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE}", "/qn", "/norestart"])
        .status();
    if let Ok(s) = status2 {
        println!("uninstall exit code: {}", s.code().unwrap_or(-1));
    }

    // Try to open with msi crate
    let cursor = std::io::Cursor::new(&msi_data);
    match msi::Package::open(cursor) {
        Ok(pkg) => {
            println!("msi crate: OK - {} tables", pkg.tables().count());
            for table in pkg.tables() {
                println!("  Table: {}", table.name());
            }
        }
        Err(e) => {
            println!("msi crate: FAILED: {}", e);
            // Write to file for inspection
            std::fs::write("C:\\temp\\custom_ole_test.msi", &msi_data).unwrap();
            println!("Wrote to C:\\temp\\custom_ole_test.msi");
        }
    }

    // Check OLE header
    println!("\nOLE header check:");
    println!("  Magic: {:02X}{:02X}{:02X}{:02X}", msi_data[0], msi_data[1], msi_data[2], msi_data[3]);
    println!("  Minor version: {}", u16::from_le_bytes([msi_data[24], msi_data[25]]));
    println!("  Major version: {}", u16::from_le_bytes([msi_data[26], msi_data[27]]));
    println!("  Byte order: {:02X}{:02X}", msi_data[28], msi_data[29]);
    println!("  Sector shift: {}", u16::from_le_bytes([msi_data[30], msi_data[31]]));
    println!("  Mini sector shift: {}", u16::from_le_bytes([msi_data[32], msi_data[33]]));

    // Check root entry CLSID (at offset 80 in first directory sector)
    let first_dir_sector = u32::from_le_bytes([msi_data[48], msi_data[49], msi_data[50], msi_data[51]]);
    let sector_size = 1 << u16::from_le_bytes([msi_data[30], msi_data[31]]);
    let dir_offset = 512 + first_dir_sector as usize * sector_size;
    println!("  First dir sector: {} (offset {})", first_dir_sector, dir_offset);
    let clsid = &msi_data[dir_offset + 80..dir_offset + 96];
    println!("  Root CLSID: {:02X}{:02X}{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}-...",
        clsid[0], clsid[1], clsid[2], clsid[3],
        clsid[4], clsid[5], clsid[6], clsid[7]);
    
    // Expected MSI CLSID: {000C1084-0000-0000-C000-000000000046}
    let expected = [0x84, 0x10, 0x0C, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46];
    if clsid == &expected {
        println!("  ✓ Root CLSID matches MSI CLSID");
    } else {
        println!("  ✗ Root CLSID does NOT match MSI CLSID");
        println!("    Expected: 84 10 0C 00 00 00 00 00 C0 00 00 00 00 00 00 46");
        println!("    Got:      {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X}",
            clsid[0], clsid[1], clsid[2], clsid[3], clsid[4], clsid[5], clsid[6], clsid[7],
            clsid[8], clsid[9], clsid[10], clsid[11], clsid[12], clsid[13], clsid[14], clsid[15]);
    }
}
