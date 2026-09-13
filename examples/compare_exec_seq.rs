use std::io::Cursor;
use std::process::Command;

fn main() {
    // === Build reference MSI with msi crate ===
    println!("=== Reference MSI (msi crate) ===");
    let ref_cursor = Cursor::new(Vec::new());
    let mut ref_pkg = msi::Package::create(msi::PackageType::Installer, ref_cursor).unwrap();

    ref_pkg.create_table("Property", vec![
        msi::Column::build("Property").primary_key().string(72),
        msi::Column::build("Value").string(1024),
    ]).unwrap();
    ref_pkg.insert_rows(msi::Insert::into("Property")
        .row(vec![msi::Value::Str("ProductName".into()), msi::Value::Str("Test".into())])
        .row(vec![msi::Value::Str("ProductCode".into()), msi::Value::Str("{12345678-1234-1234-1234-123456789012}".into())])
    ).unwrap();

    ref_pkg.create_table("Directory", vec![
        msi::Column::build("Directory").primary_key().category(msi::Category::Identifier).string(72),
        msi::Column::build("Directory_Parent").nullable().category(msi::Category::Identifier).string(72),
        msi::Column::build("DefaultDir").primary_key().category(msi::Category::DefaultDir).string(255),
    ]).unwrap();
    ref_pkg.insert_rows(msi::Insert::into("Directory")
        .row(vec![msi::Value::Str("TARGETDIR".into()), msi::Value::Null, msi::Value::Str("VelTest:VelTest".into())])
        .row(vec![msi::Value::Str("INSTALLDIR".into()), msi::Value::Str("TARGETDIR".into()), msi::Value::Str("VelTest".into())])
    ).unwrap();

    ref_pkg.create_table("InstallExecuteSequence", vec![
        msi::Column::build("Action").primary_key().string(72),
        msi::Column::build("Condition").nullable().string(255),
        msi::Column::build("Sequence").nullable().int16(),
    ]).unwrap();
    ref_pkg.insert_rows(msi::Insert::into("InstallExecuteSequence")
        .row(vec![msi::Value::Str("CostInitialize".into()), msi::Value::Null, msi::Value::Int(800)])
        .row(vec![msi::Value::Str("CostFinalize".into()), msi::Value::Null, msi::Value::Int(1000)])
    ).unwrap();

    let ref_data = ref_pkg.into_inner().unwrap().into_inner();
    std::fs::write("ref_exec_seq.msi", &ref_data).unwrap();
    println!("Reference MSI: {} bytes", ref_data.len());

    // === Build velocity-msi version ===
    println!("\n=== Velocity MSI ===");
    let mut builder = velocity_msi::MsiBuilder::new();
    builder.set_title("VelocityTest");
    builder.set_author("Test");
    builder.set_template("Intel", 1033);

    builder.create_table("Property", vec![
        velocity_msi::Column::build("Property").string(72).primary_key().build(),
        velocity_msi::Column::build("Value").string(1024).build(),
    ]).unwrap();
    builder.insert_rows("Property", vec![
        vec![velocity_msi::Value::from("ProductName"), velocity_msi::Value::from("Test")],
        vec![velocity_msi::Value::from("ProductCode"), velocity_msi::Value::from("{12345678-1234-1234-1234-123456789012}")],
    ]).unwrap();

    builder.create_table("Directory", vec![
        velocity_msi::Column::build("Directory").string(72).primary_key().category("Identifier").build(),
        velocity_msi::Column::build("Directory_Parent").string(72).nullable().foreign_key("Directory", 1).category("Identifier").build(),
        velocity_msi::Column::build("DefaultDir").string(255).primary_key().category("DefaultDir").build(),
    ]).unwrap();
    builder.insert_rows("Directory", vec![
        vec![velocity_msi::Value::from("TARGETDIR"), velocity_msi::Value::Null, velocity_msi::Value::from("VelTest:VelTest")],
        vec![velocity_msi::Value::from("INSTALLDIR"), velocity_msi::Value::from("TARGETDIR"), velocity_msi::Value::from("VelTest")],
    ]).unwrap();

    builder.create_table("InstallExecuteSequence", vec![
        velocity_msi::Column::build("Action").string(72).primary_key().build(),
        velocity_msi::Column::build("Condition").string(255).nullable().build(),
        velocity_msi::Column::build("Sequence").int16().nullable().build(),
    ]).unwrap();
    builder.insert_rows("InstallExecuteSequence", vec![
        vec![velocity_msi::Value::from("CostInitialize"), velocity_msi::Value::Null, velocity_msi::Value::Int(800)],
        vec![velocity_msi::Value::from("CostFinalize"), velocity_msi::Value::Null, velocity_msi::Value::Int(1000)],
    ]).unwrap();

    let vel_data = builder.build().unwrap();
    std::fs::write("vel_exec_seq.msi", &vel_data).unwrap();
    println!("Velocity MSI: {} bytes", vel_data.len());

    // === Compare _Columns bitfields ===
    println!("\n=== Reference _Columns for Directory ===");
    let ref_cursor2 = Cursor::new(ref_data.clone());
    let mut ref_pkg2 = msi::Package::open(ref_cursor2).unwrap();
    for row in ref_pkg2.select_rows(msi::Select::table("_Columns")).unwrap() {
        let table = row[0].as_str().unwrap_or("?");
        if table == "Directory" {
            let number = row[1].as_int().unwrap_or(-1);
            let name = row[2].as_str().unwrap_or("?");
            let type_val = row[3].as_int().unwrap_or(-1);
            println!("  Col {}: {} = 0x{:08X} ({:032b})", number, name, type_val, type_val);
        }
    }

    println!("\n=== Velocity _Columns for Directory (read via msi crate) ===");
    let vel_cursor = Cursor::new(vel_data.clone());
    match msi::Package::open(vel_cursor) {
        Ok(mut vel_pkg) => {
            for row in vel_pkg.select_rows(msi::Select::table("_Columns")).unwrap() {
                let table = row[0].as_str().unwrap_or("?");
                if table == "Directory" {
                    let number = row[1].as_int().unwrap_or(-1);
                    let name = row[2].as_str().unwrap_or("?");
                    let type_val = row[3].as_int().unwrap_or(-1);
                    println!("  Col {}: {} = 0x{:08X} ({:032b})", number, name, type_val, type_val);
                }
            }

            println!("\n=== Reference _Validation for Directory ===");
            for row in vel_pkg.select_rows(msi::Select::table("_Validation")).unwrap() {
                let table = row[0].as_str().unwrap_or("?");
                if table == "Directory" {
                    let col = row[1].as_str().unwrap_or("?");
                    let nullable = row[2].as_str().unwrap_or("?");
                    let category = row[7].as_str().unwrap_or("NULL");
                    println!("  {} nullable={} category={}", col, nullable, category);
                }
            }
        }
        Err(e) => {
            println!("  ERROR opening velocity MSI with msi crate: {}", e);
            println!("  This means the OLE structure is broken!");
        }
    }

    // === Compare _Validation ===
    println!("\n=== Reference _Validation for Directory ===");
    for row in ref_pkg2.select_rows(msi::Select::table("_Validation")).unwrap() {
        let table = row[0].as_str().unwrap_or("?");
        if table == "Directory" {
            let col = row[1].as_str().unwrap_or("?");
            let nullable = row[2].as_str().unwrap_or("?");
            let min_val = row[3].as_int().unwrap_or(0);
            let max_val = row[4].as_int().unwrap_or(0);
            let key_table = row[5].as_str().unwrap_or("NULL");
            let key_col = row[6].as_int().unwrap_or(0);
            let category = row[7].as_str().unwrap_or("NULL");
            let set = row[8].as_str().unwrap_or("NULL");
            println!("  {} nullable={} min={} max={} key_table={} key_col={} category={} set={}",
                col, nullable, min_val, max_val, key_table, key_col, category, set);
        }
    }

    // === Test both with msiexec ===
    println!("\n=== Testing reference MSI ===");
    let _ = std::fs::remove_file("ref_exec_seq.log");
    let output = Command::new("msiexec")
        .args(["/i", "ref_exec_seq.msi", "/qn", "/norestart", "/lv", "ref_exec_seq.log"])
        .output().unwrap();
    println!("Exit code: {}", output.status.code().unwrap_or(-1));

    println!("\n=== Testing velocity MSI ===");
    let _ = std::fs::remove_file("vel_exec_seq.log");
    let output = Command::new("msiexec")
        .args(["/i", "vel_exec_seq.msi", "/qn", "/norestart", "/lv", "vel_exec_seq.log"])
        .output().unwrap();
    println!("Exit code: {}", output.status.code().unwrap_or(-1));

    // Check logs for errors
    for (name, logfile) in &[("reference", "ref_exec_seq.log"), ("velocity", "vel_exec_seq.log")] {
        if let Ok(log) = std::fs::read_to_string(logfile) {
            for line in log.lines() {
                if line.contains("2705") || line.contains("1620") || line.contains("DEBUG: Error") {
                    println!("  [{}] {}", name, line.trim());
                }
            }
        }
    }

    // === Dump Directory stream hex from both ===
    println!("\n=== Directory stream comparison ===");
    dump_directory_stream("ref_exec_seq.msi", "reference");
    dump_directory_stream("vel_exec_seq.msi", "velocity");
}

fn dump_directory_stream(path: &str, label: &str) {
    let data = std::fs::read(path).unwrap();
    let cursor = Cursor::new(&data);
    match cfb::CompoundFile::open(cursor) {
        Ok(mut cfb) => {
            let stream_names: Vec<String> = cfb.walk()
                .filter(|e| e.is_stream())
                .map(|e| e.name().to_string())
                .collect();
            for name in &stream_names {
                let decoded = decode_stream_name(name);
                if decoded.as_deref() == Some("Directory") {
                    let mut stream_data = Vec::new();
                    let mut stream = cfb.open_stream(name).unwrap();
                    std::io::Read::read_to_end(&mut stream, &mut stream_data).unwrap();
                    println!("  [{}] Directory stream ({} bytes): {:02X?}", label, stream_data.len(), stream_data);
                }
            }
        }
        Err(e) => println!("  [{}] ERROR opening OLE: {}", label, e),
    }
}

fn decode_stream_name(encoded: &str) -> Option<String> {
    let chars: Vec<char> = encoded.chars().collect();
    if chars.is_empty() || chars[0] != '\u{4840}' { return None; }
    let mut decoded = String::new();
    let mut idx = 1;
    while idx < chars.len() {
        let c = chars[idx] as u32;
        if c >= 0x3800 && c < 0x4800 {
            let val = (c - 0x3800) as u16;
            let v1 = (val & 0x3F) as u8;
            let v2 = (val >> 6) as u8;
            decoded.push(from_b64(v1));
            decoded.push(from_b64(v2));
            idx += 1;
        } else if c >= 0x4800 && c < 0x4840 {
            decoded.push(from_b64((c - 0x4800) as u8));
            idx += 1;
        } else {
            decoded.push(chars[idx]);
            idx += 1;
        }
    }
    Some(decoded)
}

fn from_b64(val: u8) -> char {
    match val {
        0..=9 => (b'0' + val) as char,
        10..=35 => (b'A' + val - 10) as char,
        36..=61 => (b'a' + val - 36) as char,
        62 => '.',
        63 => '_',
        _ => '?',
    }
}
