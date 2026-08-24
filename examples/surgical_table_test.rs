/// Surgical test: take working template MSI, replace ONLY user table streams
/// with our serialized data. Keep system tables and string pool from template.
/// This isolates whether our table serialization is correct.
///
/// cargo run --example surgical_table_test -p velocity-msi
use std::io::{Cursor, Read, Write};
use velocity_msi::{MsiBuilder, Column, Value};

fn main() {
    println!("=== SURGICAL TABLE REPLACEMENT TEST ===\n");

    let _ = std::process::Command::new("taskkill")
        .args(&["/F", "/IM", "msiexec.exe"]).output();
    std::thread::sleep(std::time::Duration::from_secs(2));

    let template_path = "C:\\Program Files\\Microsoft Office\\root\\Integration\\C2RIntLoc.en-us.16.msi";
    let template_data = std::fs::read(template_path).unwrap();
    println!("Template: {} bytes", template_data.len());

    // Read all template streams
    let mut template_comp = cfb::CompoundFile::open(Cursor::new(&template_data)).unwrap();
    let mut all_streams: Vec<(String, Vec<u8>)> = Vec::new();
    let names: Vec<String> = template_comp.walk()
        .filter(|e| e.is_stream())
        .map(|e| e.name().to_string())
        .collect();
    for name in &names {
        let mut stream = template_comp.open_stream(name).unwrap();
        let mut data = Vec::new();
        stream.read_to_end(&mut data).unwrap();
        all_streams.push((name.clone(), data));
    }
    drop(template_comp);
    println!("Template streams: {}", all_streams.len());

    // Identify system vs user streams
    // System streams: _Tables, _Columns, _Validation, _StringPool, _StringData, SummaryInformation, DigitalSignature, MsiDigitalSignatureEx
    // User streams: encoded table names (all start with the table prefix \u{4840})
    let system_stream_names = [
        "\u{0005}SummaryInformation",
        "\u{0005}DigitalSignature",
        "\u{0005}MsiDigitalSignatureEx",
    ];

    // The encoded system table names start with \u{4840} prefix
    // _Tables, _Columns, _Validation all have this prefix
    // Let's identify them by trying to decode

    // Build a velocity-msi with a simple Property table
    let mut builder = MsiBuilder::new();
    builder.set_title("Surgical Test");
    builder.set_author("Velocity");

    builder.create_table("Property", vec![
        Column::build("Property").string(72).primary_key().build(),
        Column::build("Value").string(255).nullable().localizable().build(),
    ]).unwrap();
    builder.insert_rows("Property", vec![
        vec![Value::from("ProductName"), Value::from("Surgical Test")],
    ]).unwrap();

    // Get our serialized Property table stream
    let our_prop_data = {
        // Build just to get the serialized table data
        let mut test_builder = MsiBuilder::new();
        test_builder.create_table("Property", vec![
            Column::build("Property").string(72).primary_key().build(),
            Column::build("Value").string(255).nullable().localizable().build(),
        ]).unwrap();
        test_builder.insert_rows("Property", vec![
            vec![Value::from("ProductName"), Value::from("Surgical Test")],
        ]).unwrap();
        // We need the raw table bytes - build the full MSI and extract
        let msi_data = test_builder.build().unwrap();
        let mut our_comp = cfb::CompoundFile::open(Cursor::new(&msi_data)).unwrap();
        let our_names: Vec<String> = our_comp.walk()
            .filter(|e| e.is_stream())
            .map(|e| e.name().to_string())
            .collect();
        // Find the Property table stream (it's a user table, no \u{4840} prefix)
        let mut prop_data = None;
        for name in &our_names {
            if !name.starts_with('\u{4840}') && !name.starts_with('\u{0005}') {
                let mut s = our_comp.open_stream(name).unwrap();
                let mut d = Vec::new();
                s.read_to_end(&mut d).unwrap();
                let safe: String = name.chars().map(|c| if c.is_ascii() && !c.is_control() { c } else { '.' }).collect();
                println!("Our stream: {} ({} bytes)", safe, d.len());
                if name == &our_names.iter().find(|n| !n.starts_with('\u{4840}') && !n.starts_with('\u{0005}')).unwrap() {
                    prop_data = Some((name.clone(), d));
                }
            }
        }
        prop_data.unwrap()
    };
    println!("Our Property stream: {} ({} bytes)", 
        our_prop_data.0.chars().map(|c| if c.is_ascii() && !c.is_control() { c } else { '.' }).collect::<String>(),
        our_prop_data.1.len());

    // Now create a new MSI: template streams + our Property stream added
    let out_path = "C:\\temp\\surgical_test.msi";
    {
        let mut buf = Vec::new();
        {
            let cursor = Cursor::new(&mut buf);
            let mut new_comp = cfb::CompoundFile::create_with_version(
                cfb::Version::V3, cursor,
            ).unwrap();

            // Set MSI CLSID
            let msi_clsid = uuid::Uuid::from_bytes([
                0x00, 0x0C, 0x10, 0x84, 0x00, 0x00, 0x00, 0x00,
                0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46,
            ]);
            new_comp.set_storage_clsid("", msi_clsid).unwrap();

            // Copy ALL template streams
            for (name, data) in &all_streams {
                let mut s = new_comp.create_stream(name).unwrap();
                s.write_all(data).unwrap();
            }

            // Add our Property stream
            let mut s = new_comp.create_stream(&our_prop_data.0).unwrap();
            s.write_all(&our_prop_data.1).unwrap();

            new_comp.flush().unwrap();
        }
        std::fs::write(out_path, &buf).unwrap();
        println!("\nSurgical MSI: {} bytes", buf.len());
    }

    // Test with msiexec
    println!("\n--- Testing surgical MSI (template + our Property stream) ---");
    let _ = std::fs::remove_file("C:\\temp\\surgical.log");
    let output = std::process::Command::new("msiexec")
        .args(&["/i", out_path, "/qn", "/l*v", "C:\\temp\\surgical.log"])
        .output().unwrap();
    let exit_code = output.status.code().unwrap_or(-1);
    println!("Exit code: {}", exit_code);
    match exit_code {
        0 => println!("SUCCESS!"),
        1603 => println!("1603 (accepted but install failed)"),
        1613 => println!("1613 (invalid package)"),
        1620 => println!("1620 (could not open)"),
        _ => println!("Error {}", exit_code),
    }

    // Now try WITHOUT adding our stream (pure template roundtrip)
    let out_path2 = "C:\\temp\\surgical_baseline.msi";
    {
        let mut buf = Vec::new();
        {
            let cursor = Cursor::new(&mut buf);
            let mut new_comp = cfb::CompoundFile::create_with_version(
                cfb::Version::V3, cursor,
            ).unwrap();
            let msi_clsid = uuid::Uuid::from_bytes([
                0x00, 0x0C, 0x10, 0x84, 0x00, 0x00, 0x00, 0x00,
                0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46,
            ]);
            new_comp.set_storage_clsid("", msi_clsid).unwrap();
            for (name, data) in &all_streams {
                let mut s = new_comp.create_stream(name).unwrap();
                s.write_all(data).unwrap();
            }
            new_comp.flush().unwrap();
        }
        std::fs::write(out_path2, &buf).unwrap();
    }

    println!("\n--- Testing baseline (pure template roundtrip, no changes) ---");
    let _ = std::fs::remove_file("C:\\temp\\surgical_baseline.log");
    let output2 = std::process::Command::new("msiexec")
        .args(&["/i", out_path2, "/qn", "/l*v", "C:\\temp\\surgical_baseline.log"])
        .output().unwrap();
    let exit2 = output2.status.code().unwrap_or(-1);
    println!("Exit code: {}", exit2);

    // Check logs
    for log_name in &["C:\\temp\\surgical.log", "C:\\temp\\surgical_baseline.log"] {
        if let Ok(log) = std::fs::read_to_string(log_name) {
            let lines: Vec<&str> = log.lines().collect();
            println!("\n--- {} highlights ---", log_name);
            for line in &lines {
                if line.contains("Error") || line.contains("return value") ||
                   line.contains("Product:") || line.contains("Installation") {
                    println!("  {}", line);
                }
            }
        }
    }

    println!("\n=== DONE ===");
}
