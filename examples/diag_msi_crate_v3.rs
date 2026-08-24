/// Create MSI with File table using msi crate, repackage as V3, compare with velocity-msi
/// cargo run --example diag_msi_crate_v3 -p velocity-msi
use std::io::{Cursor, Read, Write};

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

fn read_all_streams(data: &[u8]) -> Vec<(String, Vec<u8>)> {
    let mut comp = cfb::CompoundFile::open(Cursor::new(data)).unwrap();
    let names: Vec<String> = comp.walk()
        .filter(|e| e.is_stream())
        .map(|e| e.path().to_str().unwrap().to_string())
        .collect();
    let mut result = Vec::new();
    for name in names {
        let mut s = comp.open_stream(&name).unwrap();
        let mut buf = Vec::new();
        s.read_to_end(&mut buf).unwrap();
        result.push((name, buf));
    }
    result
}

fn main() {
    let pc = make_uuid();
    let uc = make_uuid();

    // Step 1: Create V4 MSI with msi crate
    println!("=== Step 1: Create V4 MSI with msi crate ===");
    let v4_data: Vec<u8> = {
        let cursor = Cursor::new(Vec::new());
        let mut package = msi::Package::create(msi::PackageType::Installer, cursor).unwrap();
        package.set_database_codepage(msi::CodePage::Windows1252);

        package.create_table("Property", vec![
            msi::Column::build("Property").primary_key().string(72),
            msi::Column::build("Value").nullable().string(255),
        ]).unwrap();
        package.insert_rows(msi::Insert::into("Property").rows(vec![
            vec![msi::Value::Str("ProductName".into()), msi::Value::Str("Test".into())],
            vec![msi::Value::Str("ProductVersion".into()), msi::Value::Str("1.0.0".into())],
            vec![msi::Value::Str("Manufacturer".into()), msi::Value::Str("V".into())],
            vec![msi::Value::Str("ProductCode".into()), msi::Value::Str(pc.clone())],
            vec![msi::Value::Str("UpgradeCode".into()), msi::Value::Str(uc.clone())],
            vec![msi::Value::Str("ProductLanguage".into()), msi::Value::Str("1033".into())],
        ])).unwrap();

        package.create_table("File", vec![
            msi::Column::build("File_").primary_key().string(72),
            msi::Column::build("Component_").string(72),
            msi::Column::build("FileName").localizable().string(255),
            msi::Column::build("FileSize").int32(),
            msi::Column::build("Attributes").nullable().int16(),
            msi::Column::build("Sequence").int16(),
        ]).unwrap();
        package.insert_rows(msi::Insert::into("File").row(vec![
            msi::Value::Str("F1".into()),
            msi::Value::Str("MC".into()),
            msi::Value::Str("test.txt".into()),
            msi::Value::Int(10),
            msi::Value::Int(0),
            msi::Value::Int(1),
        ])).unwrap();

        package.flush().unwrap();
        package.into_inner().unwrap().into_inner()
    };
    println!("V4 MSI size: {} bytes", v4_data.len());

    // Step 2: Test V4 directly (expect 1620)
    println!("\n=== Step 2: Test V4 directly ===");
    {
        let _ = std::fs::create_dir_all("C:\\temp");
        let path = "C:\\temp\\test_v4.msi";
        let _ = std::fs::remove_file(path);
        std::fs::write(path, &v4_data).unwrap();
        let output = std::process::Command::new("msiexec")
            .args(&["/i", path, "/qn"])
            .output().unwrap();
        println!("V4 exit code: {}", output.status.code().unwrap_or(-1));
    }

    // Step 3: Repackage V4 → V3
    println!("\n=== Step 3: Repackage V4 → V3 ===");
    let v3_data: Vec<u8> = {
        // Read all streams from V4
        let streams = read_all_streams(&v4_data);

        let mut v3_buf = Vec::new();
        {
            let dst_cursor = Cursor::new(&mut v3_buf);
            let mut dst = cfb::CompoundFile::create_with_version(
                cfb::Version::V3, dst_cursor,
            ).unwrap();

            let msi_clsid = uuid::Uuid::from_bytes([
                0x00, 0x0C, 0x10, 0x84, 0x00, 0x00, 0x00, 0x00,
                0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46,
            ]);
            dst.set_storage_clsid("", msi_clsid).unwrap();

            for (name, data) in &streams {
                let mut s = dst.create_stream(name).unwrap();
                s.write_all(data).unwrap();
            }
            dst.flush().unwrap();
        }
        v3_buf
    };
    println!("V3 repackaged size: {} bytes", v3_data.len());

    // Step 4: Test V3 repackaged
    println!("\n=== Step 4: Test V3 repackaged ===");
    {
        let _ = std::fs::create_dir_all("C:\\temp");
        let path = "C:\\temp\\test_v3_repack.msi";
        let log_path = "C:\\temp\\test_v3_repack.log";
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(log_path);
        std::fs::write(path, &v3_data).unwrap();
        let output = std::process::Command::new("msiexec")
            .args(&["/i", path, "/qn", "/l*v", log_path])
            .output().unwrap();
        let ec = output.status.code().unwrap_or(-1);
        println!("V3 repackaged exit code: {}", ec);
        if ec != 0 {
            if let Ok(log) = std::fs::read_to_string(log_path) {
                for line in log.lines() {
                    if line.contains("Note:") || line.contains("Error") || line.contains("2725")
                    {
                        println!("  LOG: {}", line.trim());
                    }
                }
            }
        } else {
            println!("SUCCESS! msi crate data works in V3!");
        }
    }

    // Step 5: Compare with velocity-msi output
    println!("\n=== Step 5: Compare msi-crate-V3 vs velocity-msi ===");
    {
        use velocity_msi::{Column, MsiBuilder, Value};

        let mut b = MsiBuilder::new();
        b.set_title("Test");
        b.set_author("V");
        b.set_template("Intel", 1033);
        b.create_table("Property", vec![
            Column::build("Property").string(72).primary_key().build(),
            Column::build("Value").nullable().string(255).localizable().build(),
        ]).unwrap();
        b.insert_rows("Property", vec![
            vec![Value::from("ProductName"), Value::from("Test")],
            vec![Value::from("ProductVersion"), Value::from("1.0.0")],
            vec![Value::from("Manufacturer"), Value::from("V")],
            vec![Value::from("ProductCode"), Value::from(pc.as_str())],
            vec![Value::from("UpgradeCode"), Value::from(uc.as_str())],
            vec![Value::from("ProductLanguage"), Value::from("1033")],
        ]).unwrap();
        b.create_table("File", vec![
            Column::build("File_").string(72).primary_key().build(),
            Column::build("Component_").string(72).build(),
            Column::build("FileName").string(255).localizable().build(),
            Column::build("FileSize").int32().build(),
            Column::build("Attributes").nullable().int16().build(),
            Column::build("Sequence").int16().build(),
        ]).unwrap();
        b.insert_rows("File", vec![
            vec![Value::from("F1"), Value::from("MC"), Value::from("test.txt"),
                 Value::Int(10), Value::Int(0), Value::Int(1)],
        ]).unwrap();
        let our_data = b.build().unwrap();

        let ref_streams = read_all_streams(&v3_data);
        let our_streams = read_all_streams(&our_data);

        println!("msi-crate-V3 streams ({}):", ref_streams.len());
        for (name, data) in &ref_streams {
            println!("  {} ({} bytes)", name, data.len());
        }
        println!("\nvelocity-msi streams ({}):", our_streams.len());
        for (name, data) in &our_streams {
            println!("  {} ({} bytes)", name, data.len());
        }

        // Compare each stream
        println!("\n=== Stream-by-stream comparison ===");
        for (our_name, our_data) in &our_streams {
            if let Some((_, ref_data)) = ref_streams.iter().find(|(n, _)| n == our_name) {
                let same = our_data == ref_data;
                println!("{}: {} vs {} bytes → {}",
                    our_name, our_data.len(), ref_data.len(),
                    if same { "SAME" } else { "DIFFERENT" });
                if !same {
                    // Show first difference
                    let min_len = our_data.len().min(ref_data.len());
                    for i in 0..min_len {
                        if our_data[i] != ref_data[i] {
                            let end = (i + 16).min(min_len);
                            println!("  First diff at byte {}: ours={:02X?} ref={:02X?}",
                                i, &our_data[i..end], &ref_data[i..end]);
                            break;
                        }
                    }
                    if our_data.len() != ref_data.len() {
                        println!("  Length diff: {} vs {}", our_data.len(), ref_data.len());
                    }
                    // For small streams, dump full hex
                    if our_data.len() <= 64 && ref_data.len() <= 64 {
                        println!("  ours: {:02X?}", our_data);
                        println!("  ref:  {:02X?}", ref_data);
                    }
                }
            } else {
                println!("{}: ONLY in velocity-msi", our_name);
            }
        }
        for (ref_name, _) in &ref_streams {
            if !our_streams.iter().any(|(n, _)| n == ref_name) {
                println!("{}: ONLY in msi-crate-V3", ref_name);
            }
        }
    }
}
