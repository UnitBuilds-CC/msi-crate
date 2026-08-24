/// Test: does an EMPTY File table (schema only, no rows) cause 2725?
/// cargo run --example diag_empty_file -p velocity-msi
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
    // TEST 1: EMPTY File table (schema only, no rows)
    println!("--- TEST 1: Empty File table (no rows) ---");
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
        // NO rows inserted
        let data = b.build().unwrap();
        test_msi("empty_file", &data);
    }

    // TEST 2: Empty Component table (no rows)
    println!("\n--- TEST 2: Empty Component table (no rows) ---");
    {
        let mut b = make_base();
        b.create_table("Component", vec![
            Column::build("Component").string(72).primary_key().build(),
            Column::build("ComponentId").nullable().string(38).build(),
            Column::build("Directory_").string(72).build(),
            Column::build("Attributes").int16().build(),
            Column::build("Condition").nullable().string(255).build(),
            Column::build("KeyPath").nullable().string(72).build(),
        ]).unwrap();
        let data = b.build().unwrap();
        test_msi("empty_component", &data);
    }

    // TEST 3: Empty File + Empty Component (both schemas, no rows)
    println!("\n--- TEST 3: Empty File + Empty Component ---");
    {
        let mut b = make_base();
        b.create_table("Component", vec![
            Column::build("Component").string(72).primary_key().build(),
            Column::build("ComponentId").nullable().string(38).build(),
            Column::build("Directory_").string(72).build(),
            Column::build("Attributes").int16().build(),
            Column::build("Condition").nullable().string(255).build(),
            Column::build("KeyPath").nullable().string(72).build(),
        ]).unwrap();
        b.create_table("File", vec![
            Column::build("File_").string(72).primary_key().build(),
            Column::build("Component_").string(72).build(),
            Column::build("FileName").string(255).localizable().build(),
            Column::build("FileSize").int32().build(),
            Column::build("Attributes").nullable().int16().build(),
            Column::build("Sequence").int16().build(),
        ]).unwrap();
        let data = b.build().unwrap();
        test_msi("empty_both", &data);
    }

    // TEST 4: Empty File + Directory with rows
    println!("\n--- TEST 4: Empty File + Directory with rows ---");
    {
        let mut b = make_base();
        b.create_table("Directory", vec![
            Column::build("Directory").string(72).primary_key().build(),
            Column::build("Directory_Parent").nullable().string(72).build(),
            Column::build("DefaultDir").string(255).localizable().build(),
        ]).unwrap();
        b.insert_rows("Directory", vec![
            vec![Value::from("TARGETDIR"), Value::Null, Value::from("C:\\Test")],
        ]).unwrap();
        b.create_table("File", vec![
            Column::build("File_").string(72).primary_key().build(),
            Column::build("Component_").string(72).build(),
            Column::build("FileName").string(255).localizable().build(),
            Column::build("FileSize").int32().build(),
            Column::build("Attributes").nullable().int16().build(),
            Column::build("Sequence").int16().build(),
        ]).unwrap();
        let data = b.build().unwrap();
        test_msi("empty_file_dir", &data);
    }

    // TEST 5: Table named "FILE" (uppercase, not standard)
    println!("\n--- TEST 5: Table named 'FILE' (uppercase) with rows ---");
    {
        let mut b = make_base();
        b.create_table("FILE", vec![
            Column::build("File_").string(72).primary_key().build(),
            Column::build("Component_").string(72).build(),
            Column::build("FileName").string(255).localizable().build(),
            Column::build("FileSize").int32().build(),
            Column::build("Attributes").nullable().int16().build(),
            Column::build("Sequence").int16().build(),
        ]).unwrap();
        b.insert_rows("FILE", vec![
            vec![Value::from("F1"), Value::from("MC"), Value::from("test.txt"),
                 Value::Int(10), Value::Int(0), Value::Int(1)],
        ]).unwrap();
        let data = b.build().unwrap();
        test_msi("uppercase_file", &data);
    }

    println!("\n=== SUMMARY ===");
    println!("T1: Empty File table → if 2725, schema presence triggers it");
    println!("T2: Empty Component → baseline for empty standard table");
    println!("T3: Empty File+Component → both empty");
    println!("T4: Empty File + Directory rows → does Dir data matter?");
    println!("T5: 'FILE' uppercase → case sensitivity check");
}
