/// Test MSI with a known-good cabinet (from makecab.exe)
/// cargo run --example good_cab_test -p velocity-msi
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

fn main() {
    println!("=== GOOD CABINET TEST (makecab reference) ===\n");

    let product_code = make_uuid();
    let upgrade_code = make_uuid();
    println!("ProductCode: {}", product_code);
    println!("UpgradeCode: {}", upgrade_code);

    // Read the known-good cabinet
    let cab_data = std::fs::read("C:\\temp\\good.cab").unwrap();
    println!("good.cab: {} bytes", cab_data.len());

    // The file in the cabinet is "testfile.txt"
    let file_content = std::fs::read("C:\\temp\\testfile.txt").unwrap();
    println!("testfile.txt: {} bytes", file_content.len());

    let mut builder = MsiBuilder::new();
    builder.set_title("Velocity Good Cab Test");
    builder.set_subject("Test Installation");
    builder.set_author("Velocity Installer");
    builder.set_template("Intel", 1033);
    builder.set_comments("Test with known-good cabinet");

    // === Property Table ===
    builder.create_table("Property", vec![
        Column::build("Property").string(72).primary_key().build(),
        Column::build("Value").nullable().string(255).localizable().build(),
    ]).unwrap();
    builder.insert_rows("Property", vec![
        vec![Value::from("ProductName"), Value::from("Velocity Good Cab Test")],
        vec![Value::from("ProductVersion"), Value::from("1.0.0")],
        vec![Value::from("Manufacturer"), Value::from("Velocity Corp")],
        vec![Value::from("ProductCode"), Value::from(product_code.as_str())],
        vec![Value::from("UpgradeCode"), Value::from(upgrade_code.as_str())],
        vec![Value::from("ProductLanguage"), Value::from("1033")],
    ]).unwrap();

    // === Directory Table ===
    builder.create_table("Directory", vec![
        Column::build("Directory").string(72).primary_key().build(),
        Column::build("Directory_Parent").nullable().string(72).build(),
        Column::build("DefaultDir").nullable().string(255).localizable().build(),
    ]).unwrap();
    builder.insert_rows("Directory", vec![
        vec![Value::from("TARGETDIR"), Value::Null, Value::from("SourceDir")],
        vec![Value::from("ProgramFilesFolder"), Value::from("TARGETDIR"), Value::from("PFiles")],
        vec![Value::from("INSTALLDIR"), Value::from("ProgramFilesFolder"), Value::from("VelGoodTest")],
    ]).unwrap();

    // === Component Table ===
    builder.create_table("Component", vec![
        Column::build("Component").string(72).primary_key().build(),
        Column::build("ComponentId").nullable().string(38).build(),
        Column::build("Directory_").string(72).build(),
        Column::build("Attributes").int16().build(),
        Column::build("Condition").nullable().string(255).build(),
        Column::build("KeyPath").nullable().string(72).build(),
    ]).unwrap();
    builder.insert_rows("Component", vec![
        vec![Value::from("MainComp"), Value::Null, Value::from("INSTALLDIR"),
             Value::Int(0), Value::Null, Value::from("MainFile")],
    ]).unwrap();

    // === Feature Table ===
    builder.create_table("Feature", vec![
        Column::build("Feature").string(38).primary_key().build(),
        Column::build("Feature_Parent").nullable().string(38).build(),
        Column::build("Title").nullable().string(64).localizable().build(),
        Column::build("Description").nullable().string(255).localizable().build(),
        Column::build("Display").nullable().int16().build(),
        Column::build("Level").int16().build(),
        Column::build("Directory_").nullable().string(72).build(),
        Column::build("Attributes").int16().build(),
    ]).unwrap();
    builder.insert_rows("Feature", vec![
        vec![Value::from("MainFeat"), Value::Null, Value::from("Complete"),
             Value::Null, Value::Null, Value::Int(1), Value::Null, Value::Int(0)],
    ]).unwrap();

    // === FeatureComponents Table ===
    builder.create_table("FeatureComponents", vec![
        Column::build("Feature_").string(38).primary_key().build(),
        Column::build("Component_").string(72).primary_key().build(),
    ]).unwrap();
    builder.insert_rows("FeatureComponents", vec![
        vec![Value::from("MainFeat"), Value::from("MainComp")],
    ]).unwrap();

    // === File Table ===
    // The file in the cabinet is "testfile.txt"
    builder.create_table("File", vec![
        Column::build("File_").string(72).primary_key().build(),
        Column::build("Component_").string(72).build(),
        Column::build("FileName").string(255).localizable().build(),
        Column::build("FileSize").nullable().int32().build(),
        Column::build("Sequence").int16().build(),
    ]).unwrap();
    builder.insert_rows("File", vec![
        vec![
            Value::from("MainFile"),
            Value::from("MainComp"),
            Value::from("testfile.txt"),
            Value::Int(file_content.len() as i32),
            Value::Int(1),
        ],
    ]).unwrap();

    // === InstallExecuteSequence Table ===
    builder.create_table("InstallExecuteSequence", vec![
        Column::build("Action").string(72).primary_key().build(),
        Column::build("Condition").nullable().string(255).build(),
        Column::build("Sequence").nullable().int16().build(),
    ]).unwrap();
    builder.insert_rows("InstallExecuteSequence", vec![
        vec![Value::from("CostInitialize"), Value::Null, Value::Int(800)],
        vec![Value::from("FileCost"), Value::Null, Value::Int(900)],
        vec![Value::from("CostFinalize"), Value::Null, Value::Int(1000)],
        vec![Value::from("InstallValidate"), Value::Null, Value::Int(1400)],
        vec![Value::from("InstallInitialize"), Value::Null, Value::Int(1500)],
        vec![Value::from("InstallFinalize"), Value::Null, Value::Int(6600)],
    ]).unwrap();

    // === InstallUISequence Table ===
    builder.create_table("InstallUISequence", vec![
        Column::build("Action").string(72).primary_key().build(),
        Column::build("Condition").nullable().string(255).build(),
        Column::build("Sequence").nullable().int16().build(),
    ]).unwrap();
    builder.insert_rows("InstallUISequence", vec![
        vec![Value::from("CostInitialize"), Value::Null, Value::Int(800)],
        vec![Value::from("CostFinalize"), Value::Null, Value::Int(1000)],
        vec![Value::from("ExecuteAction"), Value::Null, Value::Int(1300)],
    ]).unwrap();

    // === Media Table ===
    builder.create_table("Media", vec![
        Column::build("DiskId").int16().primary_key().build(),
        Column::build("LastSequence").int16().build(),
        Column::build("DiskPrompt").nullable().string(64).localizable().build(),
        Column::build("Cabinet").nullable().string(255).build(),
        Column::build("VolumeLabel").nullable().string(32).localizable().build(),
        Column::build("Source").nullable().string(72).build(),
    ]).unwrap();
    builder.insert_rows("Media", vec![
        vec![Value::Int(1), Value::Int(1), Value::Null,
             Value::from("#goodcab.cab"), Value::Null, Value::Null],
    ]).unwrap();

    // Embed the known-good cabinet
    // The # prefix in Media.Cabinet means "embedded" but the stream name
    // should NOT include the # prefix.
    println!("Embedding cabinet: goodcab.cab ({} bytes)", cab_data.len());
    builder.add_stream("goodcab.cab".to_string(), cab_data);

    // Build the MSI
    let msi_data = builder.build().unwrap();
    let path = "C:\\temp\\good_cab_test.msi";
    let log_path = "C:\\temp\\good_cab_test.log";
    let _ = std::fs::remove_file(path);
    let _ = std::fs::remove_file(log_path);
    std::fs::write(path, &msi_data).unwrap();
    println!("\nMSI written: {} bytes", msi_data.len());

    // Test with msiexec
    println!("\n--- msiexec install test ---");
    let output = std::process::Command::new("msiexec")
        .args(&["/i", path, "/qn", "/l*v", log_path])
        .output().unwrap();
    let ec = output.status.code().unwrap_or(-1);
    println!("Exit code: {}", ec);
    match ec {
        0 => println!("SUCCESS! MSI installed!"),
        1603 => println!("1603: Fatal error"),
        _ => println!("Error {}", ec),
    }

    // Check if files were installed
    let install_dir = "C:\\Program Files (x86)\\VelGoodTest";
    if std::path::Path::new(install_dir).exists() {
        println!("\n--- Installed files ---");
        if let Ok(entries) = std::fs::read_dir(install_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let size = entry.metadata().ok().map(|m| m.len()).unwrap_or(0);
                println!("  {} ({} bytes)", path.display(), size);
            }
        }
    } else {
        println!("\nInstall dir not found: {}", install_dir);
    }

    // Read log
    if let Ok(log) = std::fs::read_to_string(log_path) {
        println!("\n--- Log highlights ---");
        let mut count = 0;
        for line in log.lines() {
            let l = line.trim();
            if (l.contains("Error") || l.contains("2725") || l.contains("2203")
                || l.contains("cabinet") || l.contains("return value 3")
                || l.contains("Product:") || l.contains("Installation"))
                && count < 30
            {
                println!("  {}", l);
                count += 1;
            }
        }
    } else {
        println!("(no log)");
    }

    // Uninstall test
    if ec == 0 {
        println!("\n--- msiexec uninstall test ---");
        let output = std::process::Command::new("msiexec")
            .args(&["/x", &product_code, "/qn"])
            .output().unwrap();
        let ec2 = output.status.code().unwrap_or(-1);
        println!("Uninstall exit code: {}", ec2);
    }

    println!("\n=== DONE ===");
}
