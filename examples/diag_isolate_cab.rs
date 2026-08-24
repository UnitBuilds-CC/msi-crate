/// Isolate: does File+Media table WITHOUT embedded cabinet cause 2725?
/// cargo run --example diag_isolate_cab -p velocity-msi
use velocity_msi::{CabinetFile, Column, MsiBuilder, Value, build_cabinet};

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
                if line.contains("Note:") || line.contains("Error") || line.contains("error")
                    || line.contains("return value 3") || line.contains("2725") || line.contains("1603")
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

fn add_install_tables(b: &mut MsiBuilder) {
    b.create_table("Directory", vec![
        Column::build("Directory").string(72).primary_key().build(),
        Column::build("Directory_Parent").nullable().string(72).build(),
        Column::build("DefaultDir").string(255).localizable().build(),
    ]).unwrap();
    b.insert_rows("Directory", vec![
        vec![Value::from("TARGETDIR"), Value::Null, Value::from("C:\\Program Files\\TestDir")],
        vec![Value::from("INSTALLDIR"), Value::from("TARGETDIR"), Value::from("TestDir:.")],
    ]).unwrap();

    let cc = make_uuid();
    b.create_table("Component", vec![
        Column::build("Component").string(72).primary_key().build(),
        Column::build("ComponentId").nullable().string(38).build(),
        Column::build("Directory_").string(72).build(),
        Column::build("Attributes").int16().build(),
        Column::build("Condition").nullable().string(255).build(),
        Column::build("KeyPath").nullable().string(72).build(),
    ]).unwrap();
    b.insert_rows("Component", vec![
        vec![Value::from("MC"), Value::from(cc.as_str()), Value::from("INSTALLDIR"),
             Value::Int(0), Value::Null, Value::from("F1")],
    ]).unwrap();

    b.create_table("Feature", vec![
        Column::build("Feature").string(38).primary_key().build(),
        Column::build("Feature_Parent").nullable().string(38).build(),
        Column::build("Title").nullable().string(64).localizable().build(),
        Column::build("Description").nullable().string(255).localizable().build(),
        Column::build("Display").nullable().int16().build(),
        Column::build("Level").int16().build(),
        Column::build("Directory_").nullable().string(72).build(),
        Column::build("Attributes").int16().build(),
    ]).unwrap();
    b.insert_rows("Feature", vec![
        vec![Value::from("Complete"), Value::Null, Value::from("Complete"),
             Value::from("Full"), Value::Int(2), Value::Int(1),
             Value::from("INSTALLDIR"), Value::Int(0)],
    ]).unwrap();

    b.create_table("FeatureComponents", vec![
        Column::build("Feature_").string(38).primary_key().build(),
        Column::build("Component_").string(72).primary_key().build(),
    ]).unwrap();
    b.insert_rows("FeatureComponents", vec![
        vec![Value::from("Complete"), Value::from("MC")],
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

    b.create_table("Media", vec![
        Column::build("DiskId").int16().primary_key().build(),
        Column::build("LastSequence").int16().build(),
        Column::build("Cabinet").nullable().string(255).build(),
        Column::build("VolumeLabel").nullable().string(32).build(),
        Column::build("Source").nullable().string(72).build(),
    ]).unwrap();
    b.insert_rows("Media", vec![
        vec![Value::Int(1), Value::Int(1),
             Value::from("#test.cab"), Value::Null, Value::Null],
    ]).unwrap();
}

fn main() {
    // TEST 1: File+Media tables, NO cabinet stream, NO sequence tables
    println!("--- TEST 1: File+Media, no cab stream, no seq tables ---");
    {
        let mut b = make_base();
        add_install_tables(&mut b);
        // No cabinet stream added
        let data = b.build().unwrap();
        test_msi("t1_no_cab_no_seq", &data);
    }

    // TEST 2: File+Media tables + cabinet stream, NO sequence tables
    println!("\n--- TEST 2: File+Media + cab stream, no seq tables ---");
    {
        let mut b = make_base();
        add_install_tables(&mut b);
        let cab = build_cabinet(&[CabinetFile {
            name: "test.txt".to_string(),
            data: b"0123456789".to_vec(),
        }]);
        b.add_stream("test.cab".to_string(), cab);
        let data = b.build().unwrap();
        test_msi("t2_cab_no_seq", &data);
    }

    // TEST 3: Dir+Comp+Feature+FC (no File, no Media) - baseline
    println!("\n--- TEST 3: Dir+Comp+Feature+FC only (no File/Media) ---");
    {
        let mut b = make_base();
        b.create_table("Directory", vec![
            Column::build("Directory").string(72).primary_key().build(),
            Column::build("Directory_Parent").nullable().string(72).build(),
            Column::build("DefaultDir").string(255).localizable().build(),
        ]).unwrap();
        b.insert_rows("Directory", vec![
            vec![Value::from("TARGETDIR"), Value::Null, Value::from("C:\\Program Files\\TestDir")],
            vec![Value::from("INSTALLDIR"), Value::from("TARGETDIR"), Value::from("TestDir:.")],
        ]).unwrap();
        let cc = make_uuid();
        b.create_table("Component", vec![
            Column::build("Component").string(72).primary_key().build(),
            Column::build("ComponentId").nullable().string(38).build(),
            Column::build("Directory_").string(72).build(),
            Column::build("Attributes").int16().build(),
            Column::build("Condition").nullable().string(255).build(),
            Column::build("KeyPath").nullable().string(72).build(),
        ]).unwrap();
        b.insert_rows("Component", vec![
            vec![Value::from("MC"), Value::from(cc.as_str()), Value::from("INSTALLDIR"),
                 Value::Int(0), Value::Null, Value::Null],
        ]).unwrap();
        b.create_table("Feature", vec![
            Column::build("Feature").string(38).primary_key().build(),
            Column::build("Feature_Parent").nullable().string(38).build(),
            Column::build("Title").nullable().string(64).localizable().build(),
            Column::build("Description").nullable().string(255).localizable().build(),
            Column::build("Display").nullable().int16().build(),
            Column::build("Level").int16().build(),
            Column::build("Directory_").nullable().string(72).build(),
            Column::build("Attributes").int16().build(),
        ]).unwrap();
        b.insert_rows("Feature", vec![
            vec![Value::from("Complete"), Value::Null, Value::from("Complete"),
                 Value::from("Full"), Value::Int(2), Value::Int(1),
                 Value::from("INSTALLDIR"), Value::Int(0)],
        ]).unwrap();
        b.create_table("FeatureComponents", vec![
            Column::build("Feature_").string(38).primary_key().build(),
            Column::build("Component_").string(72).primary_key().build(),
        ]).unwrap();
        b.insert_rows("FeatureComponents", vec![
            vec![Value::from("Complete"), Value::from("MC")],
        ]).unwrap();
        let data = b.build().unwrap();
        test_msi("t3_no_file_no_media", &data);
    }

    // TEST 4: File table ONLY (no Media, no supporting tables)
    println!("\n--- TEST 4: File table ONLY (no Media, no support) ---");
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
        test_msi("t4_file_only", &data);
    }

    // TEST 5: Media table ONLY
    println!("\n--- TEST 5: Media table ONLY ---");
    {
        let mut b = make_base();
        b.create_table("Media", vec![
            Column::build("DiskId").int16().primary_key().build(),
            Column::build("LastSequence").int16().build(),
            Column::build("Cabinet").nullable().string(255).build(),
            Column::build("VolumeLabel").nullable().string(32).build(),
            Column::build("Source").nullable().string(72).build(),
        ]).unwrap();
        b.insert_rows("Media", vec![
            vec![Value::Int(1), Value::Int(1),
                 Value::from("#test.cab"), Value::Null, Value::Null],
        ]).unwrap();
        let data = b.build().unwrap();
        test_msi("t5_media_only", &data);
    }

    println!("\n=== SUMMARY ===");
    println!("T1: File+Media no cab no seq → if 2725, tables themselves cause it");
    println!("T2: File+Media+cab no seq → if 2725, cab doesn't help");
    println!("T3: Dir+Comp+Feature+FC only → baseline (should be 0)");
    println!("T4: File only → if 2725, File table alone triggers it");
    println!("T5: Media only → if 2725, Media table alone triggers it");
}
