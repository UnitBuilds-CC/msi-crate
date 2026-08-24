/// Test: Does _Validation cause the "File" table to fail?
/// cargo run --example diag_file_validation -p velocity-msi
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
    if ec != 0 {
        if let Ok(log) = std::fs::read_to_string(&log_path) {
            for line in log.lines() {
                if line.contains("Note:") || line.contains("error") || line.contains("Error")
                    || line.contains("return value 3")
                {
                    println!("    LOG: {}", line.trim());
                }
            }
        }
    }
    ec
}

fn make_base_builder() -> MsiBuilder {
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
    // TEST 1: File table with NO categories (current behavior - should fail 1603)
    println!("--- TEST 1: File table, no categories (baseline) ---");
    {
        let mut b = make_base_builder();
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
        test_msi("test_val1_no_cat", &data);
    }

    // TEST 2: File table WITH proper categories per MSI spec
    println!("\n--- TEST 2: File table WITH proper categories ---");
    {
        let mut b = make_base_builder();
        b.create_table("File", vec![
            Column::build("File_").string(72).primary_key().category("Identifier").build(),
            Column::build("Component_").string(72).category("Identifier").build(),
            Column::build("FileName").string(255).localizable().category("Filename").build(),
            Column::build("FileSize").int32().build(),
            Column::build("Attributes").nullable().int16().build(),
            Column::build("Sequence").int16().build(),
        ]).unwrap();
        b.insert_rows("File", vec![
            vec![Value::from("MainFile"), Value::from("MainComp"),
                 Value::from("testfile.txt"), Value::Int(23), Value::Int(0), Value::Int(1)],
        ]).unwrap();
        let data = b.build().unwrap();
        test_msi("test_val2_cats", &data);
    }

    // TEST 3: File table with categories + foreign keys per MSI spec
    println!("\n--- TEST 3: File table WITH categories + KeyTable ---");
    {
        // We can't set foreign keys in velocity-msi yet, but let's try
        // setting KeyTable in _Validation manually via a custom approach.
        // For now, just test with categories only.
        let mut b = make_base_builder();
        b.create_table("File", vec![
            Column::build("File_").string(72).primary_key().category("Identifier").build(),
            Column::build("Component_").string(72).category("Identifier").build(),
            Column::build("FileName").string(255).localizable().category("Filename").build(),
            Column::build("FileSize").int32().category("Integer").build(),
            Column::build("Attributes").nullable().int16().category("Integer").build(),
            Column::build("Sequence").int16().category("Integer").build(),
        ]).unwrap();
        b.insert_rows("File", vec![
            vec![Value::from("MainFile"), Value::from("MainComp"),
                 Value::from("testfile.txt"), Value::Int(23), Value::Int(0), Value::Int(1)],
        ]).unwrap();
        let data = b.build().unwrap();
        test_msi("test_val3_cats_int", &data);
    }

    // TEST 4: Rename "File" to "FileX" to confirm it's specifically "File"
    println!("\n--- TEST 4: 'FileX' table (not standard name, no categories) ---");
    {
        let mut b = make_base_builder();
        b.create_table("FileX", vec![
            Column::build("File_").string(72).primary_key().build(),
            Column::build("Component_").string(72).build(),
            Column::build("FileName").string(255).localizable().build(),
            Column::build("FileSize").int32().build(),
            Column::build("Attributes").nullable().int16().build(),
            Column::build("Sequence").int16().build(),
        ]).unwrap();
        b.insert_rows("FileX", vec![
            vec![Value::from("MainFile"), Value::from("MainComp"),
                 Value::from("testfile.txt"), Value::Int(23), Value::Int(0), Value::Int(1)],
        ]).unwrap();
        let data = b.build().unwrap();
        test_msi("test_val4_filex", &data);
    }

    println!("\n=== SUMMARY ===");
    println!("Test 1: Baseline (no cats) - expect 1603");
    println!("Test 2: With categories - if 0, categories fix the issue");
    println!("Test 3: With int categories - if 0, all cols need categories");
    println!("Test 4: FileX rename - if 0, confirms 'File' name is the trigger");
}
