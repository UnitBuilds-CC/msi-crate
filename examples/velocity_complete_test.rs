/// Test: Complete MSI with File/Media tables using ONLY velocity-msi
/// cargo run --example velocity_complete_test -p velocity-msi
use std::io::{Cursor, Write};
use velocity_msi::{Column, MsiBuilder, Value, CabinetFile, build_cabinet};

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
    println!("=== VELOCITY-MSI COMPLETE INSTALL TEST ===\n");

    let pc = make_uuid();
    let uc = make_uuid();
    let file_content = b"Hello from velocity-msi complete test!\n";

    // Use known-good cabinet from makecab.exe
    let cab_data = std::fs::read("C:\\temp\\good.cab").expect("read good.cab");
    println!("Cabinet (makecab): {} bytes", cab_data.len());

    // Build MSI with ALL tables
    let mut builder = MsiBuilder::new();
    builder.set_title("Velocity Complete Test");
    builder.set_author("Velocity");
    builder.set_template("Intel", 1033);

    // Property
    builder.create_table("Property", vec![
        Column::build("Property").string(72).primary_key().build(),
        Column::build("Value").nullable().string(255).localizable().build(),
    ]).unwrap();
    builder.insert_rows("Property", vec![
        vec![Value::from("ProductName"), Value::from("Velocity Complete Test")],
        vec![Value::from("ProductVersion"), Value::from("1.0.0")],
        vec![Value::from("Manufacturer"), Value::from("Velocity")],
        vec![Value::from("ProductCode"), Value::from(pc.as_str())],
        vec![Value::from("UpgradeCode"), Value::from(uc.as_str())],
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
        vec![Value::from("INSTALLDIR"), Value::from("ProgramFilesFolder"), Value::from("VelComplete")],
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

    // File table
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

    // Media table - embedded cabinet
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

    // Embed cabinet as OLE stream (name WITHOUT # - the # in Media value
    // means "embedded" but the stream name itself doesn't include #)
    builder.add_stream("velcab.cab".to_string(), cab_data.clone());

    // Build
    let msi_data = builder.build().unwrap();
    println!("MSI size: {} bytes", msi_data.len());

    // Write to file
    let _ = std::fs::create_dir_all("C:\\temp");
    let path = "C:\\temp\\velocity_complete.msi";
    let log_path = "C:\\temp\\velocity_complete.log";
    let _ = std::fs::remove_file(path);
    let _ = std::fs::remove_file(log_path);
    std::fs::write(path, &msi_data).unwrap();

    // List streams
    println!("\nStreams:");
    let cursor = Cursor::new(&msi_data);
    let comp = cfb::CompoundFile::open(cursor).unwrap();
    let entries: Vec<(String, u64)> = comp.walk()
        .filter(|e| e.is_stream())
        .map(|e| (e.name().to_string(), e.len()))
        .collect();
    for (name, size) in &entries {
        println!("  '{}' ({} bytes)", name, size);
    }

    // Test with msiexec
    println!("\nTesting with msiexec...");
    let output = std::process::Command::new("msiexec")
        .args(&["/i", path, "/qn", "/l*v", log_path])
        .output().unwrap();
    let ec = output.status.code().unwrap_or(-1);
    println!("Exit code: {}", ec);

    if ec == 0 {
        println!("\nSUCCESS! Files installed!");
        let install_dir = "C:\\Program Files (x86)\\VelComplete";
        if std::path::Path::new(install_dir).exists() {
            if let Ok(entries) = std::fs::read_dir(install_dir) {
                for entry in entries.flatten() {
                    println!("  INSTALLED: {} ({} bytes)",
                        entry.file_name().to_string_lossy(),
                        entry.metadata().map(|m| m.len()).unwrap_or(0));
                }
            }
        } else {
            println!("WARNING: Install dir not found!");
        }
        // Uninstall
        println!("\nUninstalling...");
        let uninst = std::process::Command::new("msiexec")
            .args(&["/x", &pc, "/qn"]).output().unwrap();
        println!("Uninstall exit: {}", uninst.status.code().unwrap_or(-1));
        // Verify cleanup
        if !std::path::Path::new(install_dir).exists() {
            println!("Cleanup verified: install dir removed");
        } else {
            println!("WARNING: install dir still exists after uninstall!");
        }
    } else {
        println!("\nFAILED! Exit code: {}", ec);
        if let Ok(log) = std::fs::read_to_string(log_path) {
            println!("\nKey log entries:");
            let mut count = 0;
            for line in log.lines() {
                if (line.contains("error") || line.contains("Error") ||
                    line.contains("2725") || line.contains("1603") ||
                    line.contains("return value 3") || line.contains("MSIINSTPROPERTY"))
                    && count < 30 {
                    println!("  {}", line);
                    count += 1;
                }
            }
        } else {
            println!("No log file at {}", log_path);
        }
    }
}
