/// Test: Can the msi crate create a working MSI with a "File" table?
/// cargo run --example diag_msi_crate_file -p velocity-msi
use std::io::Cursor;

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

fn make_props() -> Vec<(String, String)> {
    let pc = make_uuid();
    let uc = make_uuid();
    vec![
        ("ProductName".into(), "Test MSI Crate".into()),
        ("ProductVersion".into(), "1.0.0".into()),
        ("Manufacturer".into(), "Test".into()),
        ("ProductCode".into(), pc),
        ("UpgradeCode".into(), uc),
        ("ProductLanguage".into(), "1033".into()),
    ]
}

fn test_msi(name: &str, data: &[u8]) -> i32 {
    let _ = std::fs::create_dir_all("C:\\temp");
    let path = format!("C:\\temp\\{}.msi", name);
    let log_path = format!("C:\\temp\\{}.log", name);
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&log_path);
    std::fs::write(&path, data).unwrap();
    let output = std::process::Command::new("msiexec")
        .args(&["/i", &path, "/qn", "/l*v", &log_path])
        .output().unwrap();
    let ec = output.status.code().unwrap_or(-1);
    println!("  msiexec exit code: {}", ec);
    if ec != 0 {
        if let Ok(log) = std::fs::read_to_string(&log_path) {
            for line in log.lines() {
                if line.contains("return value 3") || line.contains("Note:") {
                    println!("    LOG: {}", line.trim());
                }
            }
        }
    }
    ec
}

fn create_msi_crate_msi(table_name: &str, with_categories: bool) -> Vec<u8> {
    let mut buf = Vec::new();
    {
        let cursor = Cursor::new(&mut buf);
        let mut package = msi::Package::create(msi::PackageType::Installer, cursor).unwrap();

        package.create_table("Property", vec![
            msi::Column::build("Property").primary_key().string(72),
            msi::Column::build("Value").nullable().localizable().string(255),
        ]).unwrap();

        if with_categories {
            package.create_table(table_name, vec![
                msi::Column::build("File_").primary_key().category(msi::Category::Identifier).string(72),
                msi::Column::build("Component_").category(msi::Category::Identifier).string(72),
                msi::Column::build("FileName").localizable().category(msi::Category::Filename).string(255),
                msi::Column::build("FileSize").int32(),
                msi::Column::build("Attributes").nullable().int16(),
                msi::Column::build("Sequence").int16(),
            ]).unwrap();
        } else {
            package.create_table(table_name, vec![
                msi::Column::build("File_").primary_key().string(72),
                msi::Column::build("Component_").string(72),
                msi::Column::build("FileName").localizable().string(255),
                msi::Column::build("FileSize").int32(),
                msi::Column::build("Attributes").nullable().int16(),
                msi::Column::build("Sequence").int16(),
            ]).unwrap();
        }

        for (name, value) in make_props() {
            package.insert_rows(
                msi::Insert::into("Property").row(vec![
                    msi::Value::Str(name),
                    msi::Value::Str(value),
                ])
            ).unwrap();
        }

        package.insert_rows(
            msi::Insert::into(table_name).row(vec![
                msi::Value::Str("MainFile".to_string()),
                msi::Value::Str("MainComp".to_string()),
                msi::Value::Str("testfile.txt".to_string()),
                msi::Value::Int(23),
                msi::Value::Int(0),
                msi::Value::Int(1),
            ])
        ).unwrap();

        package.flush().unwrap();
    }
    buf
}

fn main() {
    // TEST 1: msi crate V4 with "File" table (no categories)
    println!("--- TEST 1: msi crate V4 'File' (no categories) ---");
    let data1 = create_msi_crate_msi("File", false);
    println!("  Size: {} bytes", data1.len());
    let ec1 = test_msi("msi_crate_file", &data1);

    // TEST 2: msi crate V4 with "MyFile" table (no categories)
    println!("\n--- TEST 2: msi crate V4 'MyFile' (no categories) ---");
    let data2 = create_msi_crate_msi("MyFile", false);
    println!("  Size: {} bytes", data2.len());
    let ec2 = test_msi("msi_crate_myfile", &data2);

    // TEST 3: msi crate V4 with "File" table + categories
    println!("\n--- TEST 3: msi crate V4 'File' (with categories) ---");
    let data3 = create_msi_crate_msi("File", true);
    println!("  Size: {} bytes", data3.len());
    let ec3 = test_msi("msi_crate_file_cats", &data3);

    // TEST 4: msi crate V4 with just Property table (baseline)
    println!("\n--- TEST 4: msi crate V4 Property-only (baseline) ---");
    let data4 = {
        let mut buf = Vec::new();
        {
            let cursor = Cursor::new(&mut buf);
            let mut package = msi::Package::create(msi::PackageType::Installer, cursor).unwrap();
            package.create_table("Property", vec![
                msi::Column::build("Property").primary_key().string(72),
                msi::Column::build("Value").nullable().localizable().string(255),
            ]).unwrap();
            for (name, value) in make_props() {
                package.insert_rows(
                    msi::Insert::into("Property").row(vec![
                        msi::Value::Str(name),
                        msi::Value::Str(value),
                    ])
                ).unwrap();
            }
            package.flush().unwrap();
        }
        buf
    };
    println!("  Size: {} bytes", data4.len());
    let ec4 = test_msi("msi_crate_property_only", &data4);

    println!("\n=== RESULT ===");
    println!("msi crate Property-only:       exit {} (baseline)", ec4);
    println!("msi crate 'File' (no cats):    exit {}", ec1);
    println!("msi crate 'MyFile' (no cats):  exit {}", ec2);
    println!("msi crate 'File' (with cats):  exit {}", ec3);

    if ec4 != 0 {
        println!("=> msi crate V4 format itself fails! Cannot use msi crate as reference.");
    }
}
