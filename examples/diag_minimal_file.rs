/// Minimal test: just Property + File table (no other tables)
/// cargo run --example diag_minimal_file -p velocity-msi
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
    ec
}

fn main() {
    let pc = make_uuid();
    let uc = make_uuid();

    // TEST A: Just Property table (should work)
    println!("--- TEST A: Property only ---");
    {
        let mut b = MsiBuilder::new();
        b.set_title("Test A");
        b.set_author("V");
        b.set_template("Intel", 1033);
        b.create_table("Property", vec![
            Column::build("Property").string(72).primary_key().build(),
            Column::build("Value").nullable().string(255).localizable().build(),
        ]).unwrap();
        b.insert_rows("Property", vec![
            vec![Value::from("ProductName"), Value::from("Test A")],
            vec![Value::from("ProductVersion"), Value::from("1.0.0")],
            vec![Value::from("Manufacturer"), Value::from("V")],
            vec![Value::from("ProductCode"), Value::from(pc.as_str())],
            vec![Value::from("UpgradeCode"), Value::from(uc.as_str())],
            vec![Value::from("ProductLanguage"), Value::from("1033")],
        ]).unwrap();
        let data = b.build().unwrap();
        println!("  Size: {} bytes", data.len());
        test_msi("test_a_prop", &data);
    }

    // TEST B: Property + File table (5 cols - no Attributes)
    println!("\n--- TEST B: Property + File (5 cols, no Attributes) ---");
    {
        let mut b = MsiBuilder::new();
        b.set_title("Test B");
        b.set_author("V");
        b.set_template("Intel", 1033);
        b.create_table("Property", vec![
            Column::build("Property").string(72).primary_key().build(),
            Column::build("Value").nullable().string(255).localizable().build(),
        ]).unwrap();
        b.insert_rows("Property", vec![
            vec![Value::from("ProductName"), Value::from("Test B")],
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
            Column::build("Sequence").int16().build(),
        ]).unwrap();
        b.insert_rows("File", vec![
            vec![Value::from("MainFile"), Value::from("MainComp"),
                 Value::from("testfile.txt"), Value::Int(23), Value::Int(1)],
        ]).unwrap();
        let data = b.build().unwrap();
        println!("  Size: {} bytes", data.len());
        test_msi("test_b_file5", &data);
    }

    // TEST C: Property + File table (6 cols - with Attributes)
    println!("\n--- TEST C: Property + File (6 cols, with Attributes) ---");
    {
        let mut b = MsiBuilder::new();
        b.set_title("Test C");
        b.set_author("V");
        b.set_template("Intel", 1033);
        b.create_table("Property", vec![
            Column::build("Property").string(72).primary_key().build(),
            Column::build("Value").nullable().string(255).localizable().build(),
        ]).unwrap();
        b.insert_rows("Property", vec![
            vec![Value::from("ProductName"), Value::from("Test C")],
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
                 Value::from("testfile.txt"), Value::Int(23),
                 Value::Int(0), Value::Int(1)],
        ]).unwrap();
        let data = b.build().unwrap();
        println!("  Size: {} bytes", data.len());
        test_msi("test_c_file6", &data);
    }

    // TEST D: Property + a simple custom table (to check if ANY new table breaks)
    println!("\n--- TEST D: Property + Custom table ---");
    {
        let mut b = MsiBuilder::new();
        b.set_title("Test D");
        b.set_author("V");
        b.set_template("Intel", 1033);
        b.create_table("Property", vec![
            Column::build("Property").string(72).primary_key().build(),
            Column::build("Value").nullable().string(255).localizable().build(),
        ]).unwrap();
        b.insert_rows("Property", vec![
            vec![Value::from("ProductName"), Value::from("Test D")],
            vec![Value::from("ProductVersion"), Value::from("1.0.0")],
            vec![Value::from("Manufacturer"), Value::from("V")],
            vec![Value::from("ProductCode"), Value::from(pc.as_str())],
            vec![Value::from("UpgradeCode"), Value::from(uc.as_str())],
            vec![Value::from("ProductLanguage"), Value::from("1033")],
        ]).unwrap();
        b.create_table("CustomTable", vec![
            Column::build("Id").string(72).primary_key().build(),
            Column::build("Data").nullable().string(255).build(),
        ]).unwrap();
        b.insert_rows("CustomTable", vec![
            vec![Value::from("Row1"), Value::from("Hello")],
        ]).unwrap();
        let data = b.build().unwrap();
        println!("  Size: {} bytes", data.len());
        test_msi("test_d_custom", &data);
    }

    // TEST E: Property + Directory (2 tables)
    println!("\n--- TEST E: Property + Directory ---");
    {
        let mut b = MsiBuilder::new();
        b.set_title("Test E");
        b.set_author("V");
        b.set_template("Intel", 1033);
        b.create_table("Property", vec![
            Column::build("Property").string(72).primary_key().build(),
            Column::build("Value").nullable().string(255).localizable().build(),
        ]).unwrap();
        b.insert_rows("Property", vec![
            vec![Value::from("ProductName"), Value::from("Test E")],
            vec![Value::from("ProductVersion"), Value::from("1.0.0")],
            vec![Value::from("Manufacturer"), Value::from("V")],
            vec![Value::from("ProductCode"), Value::from(pc.as_str())],
            vec![Value::from("UpgradeCode"), Value::from(uc.as_str())],
            vec![Value::from("ProductLanguage"), Value::from("1033")],
        ]).unwrap();
        b.create_table("Directory", vec![
            Column::build("Directory").string(72).primary_key().build(),
            Column::build("Directory_Parent").nullable().string(72).build(),
            Column::build("DefaultDir").nullable().string(255).localizable().build(),
        ]).unwrap();
        b.insert_rows("Directory", vec![
            vec![Value::from("TARGETDIR"), Value::Null, Value::from("SourceDir")],
        ]).unwrap();
        let data = b.build().unwrap();
        println!("  Size: {} bytes", data.len());
        test_msi("test_e_dir", &data);
    }

    println!("\nDone. If D fails but A passes: adding ANY table breaks.");
    println!("If D passes but B/C fail: File table specifically is broken.");
}
