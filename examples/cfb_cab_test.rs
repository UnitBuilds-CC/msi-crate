/// Test: manually add cabinet stream using cfb crate directly
/// This bypasses velocity-msi's add_stream to isolate the issue.
/// cargo run --example cfb_cab_test -p velocity-msi
use std::io::{Cursor, Write};
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
    println!("=== CFB DIRECT CABINET TEST ===\n");

    let product_code = make_uuid();
    let upgrade_code = make_uuid();
    println!("ProductCode: {}", product_code);

    // Read the known-good cabinet
    let cab_data = std::fs::read("C:\\temp\\good.cab").unwrap();
    let file_content = std::fs::read("C:\\temp\\testfile.txt").unwrap();
    println!("good.cab: {} bytes, testfile.txt: {} bytes", cab_data.len(), file_content.len());

    // Build MSI WITHOUT cabinet stream
    let mut builder = MsiBuilder::new();
    builder.set_title("Velocity CFB Direct Test");
    builder.set_subject("Test");
    builder.set_author("Velocity");
    builder.set_template("Intel", 1033);

    // Property
    builder.create_table("Property", vec![
        Column::build("Property").string(72).primary_key().build(),
        Column::build("Value").nullable().string(255).localizable().build(),
    ]).unwrap();
    builder.insert_rows("Property", vec![
        vec![Value::from("ProductName"), Value::from("Velocity CFB Direct")],
        vec![Value::from("ProductVersion"), Value::from("1.0.0")],
        vec![Value::from("Manufacturer"), Value::from("Velocity")],
        vec![Value::from("ProductCode"), Value::from(product_code.as_str())],
        vec![Value::from("UpgradeCode"), Value::from(upgrade_code.as_str())],
        vec![Value::from("ProductLanguage"), Value::from("1033")],
    ]).unwrap();

    // Directory
    builder.create_table("Directory", vec![
        Column::build("Directory").string(72).primary_key().build(),
        Column::build("Directory_Parent").nullable().string(72).build(),
        Column::build("DefaultDir").nullable().string(255).localizable().build(),
    ]).unwrap();
    builder.insert_rows("Directory", vec![
        vec![Value::from("TARGETDIR"), Value::Null, Value::from("SourceDir")],
        vec![Value::from("ProgramFilesFolder"), Value::from("TARGETDIR"), Value::from("PFiles")],
        vec![Value::from("INSTALLDIR"), Value::from("ProgramFilesFolder"), Value::from("VelCfbTest")],
    ]).unwrap();

    // Component
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

    // Feature
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

    // FeatureComponents
    builder.create_table("FeatureComponents", vec![
        Column::build("Feature_").string(38).primary_key().build(),
        Column::build("Component_").string(72).primary_key().build(),
    ]).unwrap();
    builder.insert_rows("FeatureComponents", vec![
        vec![Value::from("MainFeat"), Value::from("MainComp")],
    ]).unwrap();

    // File
    builder.create_table("File", vec![
        Column::build("File_").string(72).primary_key().build(),
        Column::build("Component_").string(72).build(),
        Column::build("FileName").string(255).localizable().build(),
        Column::build("FileSize").nullable().int32().build(),
        Column::build("Sequence").int16().build(),
    ]).unwrap();
    builder.insert_rows("File", vec![
        vec![Value::from("MainFile"), Value::from("MainComp"),
             Value::from("testfile.txt"), Value::Int(file_content.len() as i32), Value::Int(1)],
    ]).unwrap();

    // InstallExecuteSequence
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

    // InstallUISequence
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

    // Media - try with # prefix (embedded cabinet)
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
             Value::from("#velcab.cab"), Value::Null, Value::Null],
    ]).unwrap();

    // Build MSI (WITHOUT cabinet stream)
    let mut msi_data = builder.build().unwrap();
    println!("MSI built: {} bytes (no cabinet stream yet)", msi_data.len());

    // Now use cfb crate to add the cabinet stream directly
    // Try multiple name variants
    let names_to_try = vec![
        "#velcab.cab",        // Raw with #
        "velcab.cab",         // Raw without #
    ];

    for (i, stream_name) in names_to_try.iter().enumerate() {
        println!("\n--- Attempt {}: stream name = '{}' ---", i+1, stream_name);
        
        // Rebuild MSI from scratch each time
        let mut msi_data = builder.build().unwrap();
        
        // Open with cfb and add the stream
        let mut buf = Vec::new();
        {
            let cursor = Cursor::new(&mut buf);
            // First copy the original MSI data
            buf = msi_data.clone();
            let cursor = Cursor::new(&mut buf);
            let mut comp = cfb::CompoundFile::open(cursor).unwrap();
            {
                let mut s = comp.create_stream(stream_name).unwrap();
                s.write_all(&cab_data).unwrap();
            }
            comp.flush().unwrap();
        }
        
        let path = format!("C:\\temp\\cfb_cab_test_{}.msi", i+1);
        let log_path = format!("C:\\temp\\cfb_cab_test_{}.log", i+1);
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(&log_path);
        std::fs::write(&path, &buf).unwrap();
        println!("MSI with cabinet: {} bytes", buf.len());
        
        // Test with msiexec
        let output = std::process::Command::new("msiexec")
            .args(&["/i", &path, "/qn", "/l*v", &log_path])
            .output().unwrap();
        let ec = output.status.code().unwrap_or(-1);
        println!("Exit code: {}", ec);
        
        if ec == 0 {
            println!("SUCCESS with stream name '{}'!", stream_name);
            // Check files
            let install_dir = "C:\\Program Files (x86)\\VelCfbTest";
            if std::path::Path::new(install_dir).exists() {
                println!("Install dir exists!");
                if let Ok(entries) = std::fs::read_dir(install_dir) {
                    for entry in entries.flatten() {
                        println!("  {}", entry.path().display());
                    }
                }
            }
            // Uninstall
            let _ = std::process::Command::new("msiexec")
                .args(&["/x", &product_code, "/qn"])
                .output();
            return;
        }
        
        // Check log
        if let Ok(log) = std::fs::read_to_string(&log_path) {
            for line in log.lines() {
                if line.contains("2725") || line.contains("cabinet") {
                    println!("  LOG: {}", line.trim());
                }
            }
        }
    }
    
    println!("\n=== ALL ATTEMPTS FAILED ===");
}
