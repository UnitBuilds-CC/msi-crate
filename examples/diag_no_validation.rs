/// Test: skip _Validation for File table to see if our validation entries conflict
/// with Windows Installer's internal schema
/// cargo run --example diag_no_validation -p velocity-msi
use velocity_msi::{Column, MsiBuilder, Value};
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

fn test_msi(label: &str, msi_data: &[u8]) -> i32 {
    let _ = std::fs::create_dir_all("C:\\temp");
    let safe = label.replace(' ', "_");
    let path = format!("C:\\temp\\{}.msi", safe);
    let log_path = format!("C:\\temp\\{}.log", safe);
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&log_path);
    std::fs::write(&path, msi_data).unwrap();
    let output = std::process::Command::new("msiexec")
        .args(&["/i", &path, "/qn", "/l*v", &log_path])
        .output().unwrap();
    let ec = output.status.code().unwrap_or(-1);
    println!("  [{}] Exit code: {}", label, ec);
    if ec != 0 {
        if let Ok(log) = std::fs::read_to_string(&log_path) {
            for line in log.lines() {
                if line.contains("Note:") || line.contains("Error") || line.contains("2725")
                {
                    println!("    LOG: {}", line.trim());
                }
            }
        }
    }
    ec
}

fn make_base() -> MsiBuilder {
    let pc = make_uuid();
    let uc = make_uuid();
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
    b
}

fn main() {
    // First, let's dump the _Columns binary for the File table
    // to understand what msiexec sees
    println!("=== Binary analysis of File table _Columns ===");
    {
        let mut b = make_base();
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
        let data = b.build().unwrap();

        // Read back with msi crate
        let cursor = Cursor::new(&data);
        let package = msi::Package::open(cursor).unwrap();

        println!("_Columns for File:");
        let rows = package.select_rows(msi::Select::table("_Columns")).unwrap();
        for row in &rows {
            if let msi::Value::Str(ref t) = *row["Table"].value() {
                if t == "File" {
                    let num = match *row["Number"].value() { msi::Value::Int(n) => n, _ => 0 };
                    let name = match *row["Name"].value() { msi::Value::Str(ref s) => s.clone(), _ => String::new() };
                    let typ = match *row["Type"].value() { msi::Value::Int(n) => n as u32, _ => 0 };
                    println!("  Col {} '{}' Type=0x{:04X}", num, name, typ);
                }
            }
        }

        println!("\n_Validation for File:");
        let rows = package.select_rows(msi::Select::table("_Validation")).unwrap();
        for row in &rows {
            if let msi::Value::Str(ref t) = *row["Table"].value() {
                if t == "File" {
                    let col = match *row["Column"].value() { msi::Value::Str(ref s) => s.clone(), _ => String::new() };
                    let nullable = match *row["Nullable"].value() { msi::Value::Str(ref s) => s.clone(), _ => String::new() };
                    let cat = match *row["Category"].value() {
                        msi::Value::Str(ref s) => format!("Some({})", s),
                        msi::Value::Null => "Null".to_string(),
                        _ => "?".to_string(),
                    };
                    println!("  Col '{}' Nullable={} Category={}", col, nullable, cat);
                }
            }
        }

        // Also check _Tables
        println!("\n_Tables entries:");
        let rows = package.select_rows(msi::Select::table("_Tables")).unwrap();
        for row in &rows {
            if let msi::Value::Str(ref t) = *row["Name"].value() {
                println!("  {}", t);
            }
        }
    }

    // Now test: what if we use the msi crate to create a V3 MSI with File table?
    // We can't (msi crate uses V4), but we CAN use msi crate to open our MSI
    // and verify everything is correct.

    // Let's try something different: create the MSI using msi crate's write API
    // and then repackage as V3
    println!("\n=== Test: msi crate V4 with File table, repackaged to V3 ===");
    {
        let pc = make_uuid();
        let uc = make_uuid();

        // Create with msi crate
        let mut buf = Vec::new();
        {
            let cursor = Cursor::new(&mut buf);
            let mut package = msi::Package::create(cursor).unwrap();

            // Property table
            {
                let mut table = package.create_table("Property", vec![
                    msi::Column::build("Property").primary_key().string(72),
                    msi::Column::build("Value").nullable().string(255),
                ]).unwrap();
                table.insert_rows(vec![
                    vec![msi::Value::Str("ProductName".into()), msi::Value::Str("Test".into())],
                    vec![msi::Value::Str("ProductVersion".into()), msi::Value::Str("1.0.0".into())],
                    vec![msi::Value::Str("Manufacturer".into()), msi::Value::Str("V".into())],
                    vec![msi::Value::Str("ProductCode".into()), msi::Value::Str(pc.clone())],
                    vec![msi::Value::Str("UpgradeCode".into()), msi::Value::Str(uc.clone())],
                    vec![msi::Value::Str("ProductLanguage".into()), msi::Value::Str("1033".into())],
                ]).unwrap();
            }

            // File table
            {
                let mut table = package.create_table("File", vec![
                    msi::Column::build("File_").primary_key().string(72),
                    msi::Column::build("Component_").string(72),
                    msi::Column::build("FileName").localizable().string(255),
                    msi::Column::build("FileSize").int32(),
                    msi::Column::build("Attributes").nullable().int16(),
                    msi::Column::build("Sequence").int16(),
                ]).unwrap();
                table.insert_rows(vec![
                    vec![
                        msi::Value::Str("F1".into()),
                        msi::Value::Str("MC".into()),
                        msi::Value::Str("test.txt".into()),
                        msi::Value::Int(10),
                        msi::Value::Int(0),
                        msi::Value::Int(1),
                    ],
                ]).unwrap();
            }

            package.flush().unwrap();
        }

        // Now repackage as V3 using cfb
        let v3_data = {
            let src = Cursor::new(&buf);
            let src_pkg = cfb::CompoundFile::open(src).unwrap();

            let mut v3_buf = Vec::new();
            {
                let dst = Cursor::new(&mut v3_buf);
                let mut dst_pkg = cfb::CompoundFile::create_with_version(cfb::Version::V3, dst).unwrap();

                // Set MSI CLSID
                let msi_clsid = uuid::Uuid::from_bytes([
                    0x00, 0x0C, 0x10, 0x84, 0x00, 0x00, 0x00, 0x00,
                    0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46,
                ]);
                dst_pkg.set_storage_clsid("", msi_clsid).unwrap();

                // Copy all streams
                fn copy_streams<F: std::io::Read + std::io::Seek + std::io::Write>(
                    src: &cfb::CompoundFile<F>,
                    dst: &mut cfb::CompoundFile<std::io::Cursor<Vec<u8>>>,
                    path: &cfb::Path,
                ) {
                    for entry in src.walk(path) {
                        if entry.is_stream() {
                            let data = src.open_stream(entry.path()).unwrap().read_all().unwrap();
                            let stream_name = entry.path().to_string();
                            // Create stream in dst
                            let mut s = dst.create_stream(&stream_name).unwrap();
                            std::io::Write::write_all(&mut s, &data).unwrap();
                        }
                    }
                }
                copy_streams(&src_pkg, &mut dst_pkg, &cfb::Path::root());
                dst_pkg.flush().unwrap();
            }
            v3_buf
        };

        println!("V3 repackaged size: {} bytes", v3_data.len());
        test_msi("msi_crate_repack", &v3_data);
    }

    println!("\n=== SUMMARY ===");
    println!("If msi_crate_repack works → our serialization is wrong");
    println!("If msi_crate_repack fails → V3 repackaging itself breaks something");
}
