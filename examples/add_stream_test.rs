/// Test: does adding a stream to a working MSI break it?
/// cargo run --example add_stream_test -p velocity-msi
use std::io::{Cursor, Write};
use velocity_msi::{Column, MsiBuilder, Value};

fn make_uuid() -> String {
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!(
        "{{{:08X}-{:04X}-4{:03X}-{:04X}-{:08X}{:04X}}}",
        (t & 0xFFFFFFFF) as u32,
        ((t >> 32) & 0xFFFF) as u16,
        ((t >> 48) & 0x0FFF) as u16,
        (((t >> 44) & 0x0FFF) as u16) | 0x8000,
        ((t >> 16) & 0xFFFFFFFF) as u32,
        (t & 0xFFFF) as u16,
    )
}

fn main() {
    println!("=== ADD STREAM TEST ===\n");

    let product_code = make_uuid();
    let upgrade_code = make_uuid();

    // Build a SIMPLE working MSI (no cabinet, no Media table)
    let mut builder = MsiBuilder::new();
    builder.set_title("Test Add Stream");
    builder.set_author("Velocity");
    builder.set_template("Intel", 1033);

    builder.create_table("Property", vec![
        Column::build("Property").string(72).primary_key().build(),
        Column::build("Value").nullable().string(255).localizable().build(),
    ]).unwrap();
    builder.insert_rows("Property", vec![
        vec![Value::from("ProductName"), Value::from("Test Add Stream")],
        vec![Value::from("ProductVersion"), Value::from("1.0.0")],
        vec![Value::from("Manufacturer"), Value::from("Velocity")],
        vec![Value::from("ProductCode"), Value::from(product_code.as_str())],
        vec![Value::from("UpgradeCode"), Value::from(upgrade_code.as_str())],
        vec![Value::from("ProductLanguage"), Value::from("1033")],
    ]).unwrap();

    // Build the MSI
    let msi_data = builder.build().unwrap();
    
    // Test 1: Original MSI (no extra streams)
    let path1 = "C:\\temp\\no_extra_stream.msi";
    std::fs::write(path1, &msi_data).unwrap();
    println!("Test 1: Original MSI (no extra stream)");
    let output = std::process::Command::new("msiexec")
        .args(&["/i", path1, "/qn"]).output().unwrap();
    let ec1 = output.status.code().unwrap_or(-1);
    println!("  Exit code: {} (0=success)", ec1);
    
    // Uninstall
    let _ = std::process::Command::new("msiexec")
        .args(&["/x", &product_code, "/qn"]).output();
    
    // Test 2: Add a dummy stream using cfb
    let pc2 = make_uuid();
    let uc2 = make_uuid();
    let mut builder2 = MsiBuilder::new();
    builder2.set_title("Test With Dummy Stream");
    builder2.set_author("Velocity");
    builder2.set_template("Intel", 1033);
    builder2.create_table("Property", vec![
        Column::build("Property").string(72).primary_key().build(),
        Column::build("Value").nullable().string(255).localizable().build(),
    ]).unwrap();
    builder2.insert_rows("Property", vec![
        vec![Value::from("ProductName"), Value::from("Test Dummy Stream")],
        vec![Value::from("ProductVersion"), Value::from("1.0.0")],
        vec![Value::from("Manufacturer"), Value::from("Velocity")],
        vec![Value::from("ProductCode"), Value::from(pc2.as_str())],
        vec![Value::from("UpgradeCode"), Value::from(uc2.as_str())],
        vec![Value::from("ProductLanguage"), Value::from("1033")],
    ]).unwrap();
    
    let msi_data2 = builder2.build().unwrap();
    
    // Open with cfb and add a dummy stream
    let mut buf = msi_data2.clone();
    {
        let cursor = Cursor::new(&mut buf);
        let mut comp = cfb::CompoundFile::open(cursor).unwrap();
        {
            let mut s = comp.create_stream("dummy_stream").unwrap();
            s.write_all(b"dummy data").unwrap();
        }
        comp.flush().unwrap();
    }
    
    let path2 = "C:\\temp\\with_dummy_stream.msi";
    std::fs::write(path2, &buf).unwrap();
    println!("\nTest 2: MSI with dummy stream added via cfb");
    let output = std::process::Command::new("msiexec")
        .args(&["/i", path2, "/qn"]).output().unwrap();
    let ec2 = output.status.code().unwrap_or(-1);
    println!("  Exit code: {} (0=success)", ec2);
    
    // Uninstall
    let _ = std::process::Command::new("msiexec")
        .args(&["/x", &pc2, "/qn"]).output();
    
    // Test 3: Add cabinet stream using cfb to a working MSI (no Media table)
    let pc3 = make_uuid();
    let uc3 = make_uuid();
    let mut builder3 = MsiBuilder::new();
    builder3.set_title("Test With Cab Stream No Media");
    builder3.set_author("Velocity");
    builder3.set_template("Intel", 1033);
    builder3.create_table("Property", vec![
        Column::build("Property").string(72).primary_key().build(),
        Column::build("Value").nullable().string(255).localizable().build(),
    ]).unwrap();
    builder3.insert_rows("Property", vec![
        vec![Value::from("ProductName"), Value::from("Test Cab No Media")],
        vec![Value::from("ProductVersion"), Value::from("1.0.0")],
        vec![Value::from("Manufacturer"), Value::from("Velocity")],
        vec![Value::from("ProductCode"), Value::from(pc3.as_str())],
        vec![Value::from("UpgradeCode"), Value::from(uc3.as_str())],
        vec![Value::from("ProductLanguage"), Value::from("1033")],
    ]).unwrap();
    
    let msi_data3 = builder3.build().unwrap();
    let cab_data = std::fs::read("C:\\temp\\good.cab").unwrap();
    
    let mut buf3 = msi_data3.clone();
    {
        let cursor = Cursor::new(&mut buf3);
        let mut comp = cfb::CompoundFile::open(cursor).unwrap();
        {
            let mut s = comp.create_stream("#velcab.cab").unwrap();
            s.write_all(&cab_data).unwrap();
        }
        comp.flush().unwrap();
    }
    
    let path3 = "C:\\temp\\with_cab_no_media.msi";
    std::fs::write(path3, &buf3).unwrap();
    println!("\nTest 3: MSI with cabinet stream but NO Media table");
    let output = std::process::Command::new("msiexec")
        .args(&["/i", path3, "/qn"]).output().unwrap();
    let ec3 = output.status.code().unwrap_or(-1);
    println!("  Exit code: {} (0=success)", ec3);
    
    // Uninstall
    let _ = std::process::Command::new("msiexec")
        .args(&["/x", &pc3, "/qn"]).output();
    
    println!("\n=== RESULTS ===");
    println!("Test 1 (no extra stream): {}", if ec1 == 0 { "PASS" } else { "FAIL" });
    println!("Test 2 (dummy stream):    {}", if ec2 == 0 { "PASS" } else { "FAIL" });
    println!("Test 3 (cab no media):    {}", if ec3 == 0 { "PASS" } else { "FAIL" });
    
    if ec1 == 0 && ec2 != 0 {
        println!("\nCONCLUSION: Adding streams via cfb BREAKS the MSI!");
    } else if ec1 == 0 && ec2 == 0 {
        println!("\nCONCLUSION: Adding streams via cfb is safe. Issue is elsewhere.");
    }
}
