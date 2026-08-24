/// Test: msi crate basic MSI (no files, no cabinet) repackaged to V3
/// cargo run --example msi_ref_basic_test -p velocity-msi
use std::io::Cursor;

fn repackage_v4_to_v3(v4_data: &[u8]) -> Vec<u8> {
    let cursor = Cursor::new(v4_data);
    let comp = cfb::CompoundFile::open(cursor).expect("open V4");
    let clsid = *comp.root_entry().clsid();
    let stream_names: Vec<String> = comp.walk()
        .filter(|e| e.is_stream())
        .map(|e| e.name().to_string())
        .collect();
    let mut comp = cfb::CompoundFile::open(Cursor::new(v4_data)).expect("reopen V4");
    let mut streams_with_data: Vec<(String, Vec<u8>)> = Vec::new();
    for name in &stream_names {
        let mut s = comp.open_stream(name.as_str()).expect("open stream");
        let mut data = Vec::new();
        std::io::Read::read_to_end(&mut s, &mut data).expect("read stream");
        streams_with_data.push((name.clone(), data));
    }
    let mut v3_buf = Vec::new();
    {
        let cursor = Cursor::new(&mut v3_buf);
        let mut v3_comp = cfb::CompoundFile::create_with_version(
            cfb::Version::V3, cursor,
        ).expect("create V3");
        v3_comp.set_storage_clsid("", clsid).expect("set clsid");
        for (name, data) in &streams_with_data {
            let mut s = v3_comp.create_stream(name).expect("create stream");
            std::io::Write::write_all(&mut s, data).expect("write stream");
        }
        v3_comp.flush().expect("flush V3");
    }
    v3_buf
}

fn make_uuid(offset: u128) -> String {
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos() + offset;
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
    println!("=== MSI CRATE BASIC (NO FILES) TEST ===\n");
    let pc = make_uuid(0);
    let uc = make_uuid(1);

    let cursor = Cursor::new(Vec::new());
    let mut package = msi::Package::create(msi::PackageType::Installer, cursor)
        .expect("create package");

    package.summary_info_mut().set_author("Velocity".to_string());
    package.summary_info_mut().set_title("Basic Ref Test".to_string());
    package.summary_info_mut().set_arch("Intel");
    package.summary_info_mut().set_languages(&[msi::Language::from_code(1033)]);

    // Property table only
    package.create_table("Property", vec![
        msi::Column::build("Property").primary_key().id_string(72),
        msi::Column::build("Value").nullable().formatted_string(255),
    ]).expect("create Property");

    package.insert_rows(msi::Insert::into("Property")
        .row(vec![msi::Value::from("ProductName"), msi::Value::from("Basic Ref Test")])
        .row(vec![msi::Value::from("ProductVersion"), msi::Value::from("1.0.0")])
        .row(vec![msi::Value::from("Manufacturer"), msi::Value::from("Velocity")])
        .row(vec![msi::Value::from("ProductCode"), msi::Value::from(pc.as_str())])
        .row(vec![msi::Value::from("UpgradeCode"), msi::Value::from(uc.as_str())])
        .row(vec![msi::Value::from("ProductLanguage"), msi::Value::from("1033")])
    ).expect("insert Property");

    package.flush().expect("flush");
    let cursor = package.into_inner().expect("into_inner");
    let v4_data = cursor.into_inner();
    println!("V4 size: {} bytes", v4_data.len());

    let msi_data = repackage_v4_to_v3(&v4_data);
    println!("V3 size: {} bytes", msi_data.len());

    let _ = std::fs::create_dir_all("C:\\temp");
    let path = "C:\\temp\\msi_ref_basic.msi";
    let log_path = "C:\\temp\\msi_ref_basic.log";
    let _ = std::fs::remove_file(path);
    let _ = std::fs::remove_file(log_path);
    std::fs::write(path, &msi_data).expect("write msi");

    // Test with msiexec
    println!("\nTest 1: Basic MSI (Property only)");
    let output = std::process::Command::new("msiexec")
        .args(&["/i", path, "/qn", "/l*v", log_path])
        .output().expect("msiexec");
    let ec = output.status.code().unwrap_or(-1);
    println!("Exit code: {}", ec);

    if let Ok(log) = std::fs::read_to_string(log_path) {
        for line in log.lines() {
            if line.contains("error") || line.contains("Error") ||
               line.contains("return value 3") {
                println!("  LOG: {}", line);
            }
        }
    }

    // Uninstall if success
    if ec == 0 {
        let _ = std::process::Command::new("msiexec")
            .args(&["/x", &pc, "/qn"]).output();
    }

    // Test 2: Also test V4 directly (without repackaging)
    println!("\nTest 2: V4 MSI directly (no repackaging)");
    let path_v4 = "C:\\temp\\msi_ref_basic_v4.msi";
    let _ = std::fs::remove_file(path_v4);
    std::fs::write(path_v4, &v4_data).expect("write v4");
    let output = std::process::Command::new("msiexec")
        .args(&["/i", path_v4, "/qn"]).output().expect("msiexec");
    let ec_v4 = output.status.code().unwrap_or(-1);
    println!("Exit code: {}", ec_v4);

    println!("\nDone!");
}
