/// Test: rename File table to see if Windows Installer validates standard table names
/// cargo run --example diag_table_name -p velocity-msi
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

fn test_msi(name: &str, msi_data: &[u8]) -> i32 {
    let _ = std::fs::create_dir_all("C:\\temp");
    let path = format!("C:\\temp\\{}.msi", name);
    let log_path = format!("C:\\temp\\{}.log", name);
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&log_path);
    std::fs::write(&path, msi_data).unwrap();
    let output = std::process::Command::new("msiexec")
        .args(&["/i", &path, "/qn", "/l*v", &log_path])
        .output().unwrap();
    let ec = output.status.code().unwrap_or(-1);
    println!("  Exit code: {}", ec);
    // Print error lines from log
    if ec != 0 {
        if let Ok(log) = std::fs::read_to_string(&log_path) {
            for line in log.lines() {
                if line.contains("Note:") || line.contains("error") || line.contains("Error") {
                    println!("    LOG: {}", line.trim());
                }
            }
        }
    }
    ec
}

fn main() {
    let pc = make_uuid();
    let uc = make_uuid();

    // TEST 1: Table named "File" with 6 cols (should fail based on previous tests)
    println!("--- TEST 1: Table 'File' (6 cols) ---");
    {
        let mut b = MsiBuilder::new();
        b.set_title("Test 1"); b.set_author("V"); b.set_template("Intel", 1033);
        b.create_table("Property", vec![
            Column::build("Property").string(72).primary_key().build(),
            Column::build("Value").nullable().string(255).localizable().build(),
        ]).unwrap();
        b.insert_rows("Property", vec![
            vec![Value::from("ProductName"), Value::from("Test 1")],
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
            vec![Value::from("MainFile"), Value::from("MainComp"),
                 Value::from("testfile.txt"), Value::Int(23), Value::Int(0), Value::Int(1)],
        ]).unwrap();
        let data = b.build().unwrap();
        test_msi("test_t1_file", &data);
    }

    // TEST 2: Same schema but table named "MyFile" (not a standard table name)
    println!("\n--- TEST 2: Table 'MyFile' (same schema as File) ---");
    {
        let mut b = MsiBuilder::new();
        b.set_title("Test 2"); b.set_author("V"); b.set_template("Intel", 1033);
        b.create_table("Property", vec![
            Column::build("Property").string(72).primary_key().build(),
            Column::build("Value").nullable().string(255).localizable().build(),
        ]).unwrap();
        b.insert_rows("Property", vec![
            vec![Value::from("ProductName"), Value::from("Test 2")],
            vec![Value::from("ProductVersion"), Value::from("1.0.0")],
            vec![Value::from("Manufacturer"), Value::from("V")],
            vec![Value::from("ProductCode"), Value::from(pc.as_str())],
            vec![Value::from("UpgradeCode"), Value::from(uc.as_str())],
            vec![Value::from("ProductLanguage"), Value::from("1033")],
        ]).unwrap();
        b.create_table("MyFile", vec![
            Column::build("File_").string(72).primary_key().build(),
            Column::build("Component_").string(72).build(),
            Column::build("FileName").string(255).localizable().build(),
            Column::build("FileSize").int32().build(),
            Column::build("Attributes").nullable().int16().build(),
            Column::build("Sequence").int16().build(),
        ]).unwrap();
        b.insert_rows("MyFile", vec![
            vec![Value::from("MainFile"), Value::from("MainComp"),
                 Value::from("testfile.txt"), Value::Int(23), Value::Int(0), Value::Int(1)],
        ]).unwrap();
        let data = b.build().unwrap();
        test_msi("test_t2_myfile", &data);
    }

    // TEST 3: Table named "Component" (another standard table name)
    println!("\n--- TEST 3: Table 'Component' (standard table) ---");
    {
        let mut b = MsiBuilder::new();
        b.set_title("Test 3"); b.set_author("V"); b.set_template("Intel", 1033);
        b.create_table("Property", vec![
            Column::build("Property").string(72).primary_key().build(),
            Column::build("Value").nullable().string(255).localizable().build(),
        ]).unwrap();
        b.insert_rows("Property", vec![
            vec![Value::from("ProductName"), Value::from("Test 3")],
            vec![Value::from("ProductVersion"), Value::from("1.0.0")],
            vec![Value::from("Manufacturer"), Value::from("V")],
            vec![Value::from("ProductCode"), Value::from(pc.as_str())],
            vec![Value::from("UpgradeCode"), Value::from(uc.as_str())],
            vec![Value::from("ProductLanguage"), Value::from("1033")],
        ]).unwrap();
        b.create_table("Component", vec![
            Column::build("Component").string(72).primary_key().build(),
            Column::build("ComponentId").nullable().string(38).build(),
            Column::build("Directory_").string(72).build(),
            Column::build("Attributes").int16().build(),
            Column::build("Condition").nullable().string(255).build(),
            Column::build("KeyPath").nullable().string(72).build(),
        ]).unwrap();
        b.insert_rows("Component", vec![
            vec![Value::from("MainComp"), Value::Null, Value::from("INSTALLDIR"),
                 Value::Int(0), Value::Null, Value::from("MainFile")],
        ]).unwrap();
        let data = b.build().unwrap();
        test_msi("test_t3_comp", &data);
    }

    // TEST 4: Table named "Media" (standard table)
    println!("\n--- TEST 4: Table 'Media' (standard table) ---");
    {
        let mut b = MsiBuilder::new();
        b.set_title("Test 4"); b.set_author("V"); b.set_template("Intel", 1033);
        b.create_table("Property", vec![
            Column::build("Property").string(72).primary_key().build(),
            Column::build("Value").nullable().string(255).localizable().build(),
        ]).unwrap();
        b.insert_rows("Property", vec![
            vec![Value::from("ProductName"), Value::from("Test 4")],
            vec![Value::from("ProductVersion"), Value::from("1.0.0")],
            vec![Value::from("Manufacturer"), Value::from("V")],
            vec![Value::from("ProductCode"), Value::from(pc.as_str())],
            vec![Value::from("UpgradeCode"), Value::from(uc.as_str())],
            vec![Value::from("ProductLanguage"), Value::from("1033")],
        ]).unwrap();
        b.create_table("Media", vec![
            Column::build("DiskId").int16().primary_key().build(),
            Column::build("LastSequence").int16().build(),
            Column::build("DiskPrompt").nullable().string(64).localizable().build(),
            Column::build("Cabinet").nullable().string(255).build(),
            Column::build("VolumeLabel").nullable().string(32).localizable().build(),
            Column::build("Source").nullable().string(72).build(),
        ]).unwrap();
        b.insert_rows("Media", vec![
            vec![Value::Int(1), Value::Int(1), Value::Null, Value::Null, Value::Null, Value::Null],
        ]).unwrap();
        let data = b.build().unwrap();
        test_msi("test_t4_media", &data);
    }

    println!("\n=== SUMMARY ===");
    println!("If Test 1 fails but Test 2 passes: 'File' name triggers schema validation");
    println!("If Test 3 fails: 'Component' name also triggers validation");
    println!("If Test 4 fails: 'Media' name also triggers validation");
    println!("If all fail: something else is wrong");
}
