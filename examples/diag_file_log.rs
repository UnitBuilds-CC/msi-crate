/// Quick test: just File table, get full msiexec log
/// cargo run --example diag_file_log -p velocity-msi
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
    let pc = make_uuid();
    let uc = make_uuid();

    let mut builder = MsiBuilder::new();
    builder.set_title("File Table Test");
    builder.set_author("Velocity");
    builder.set_template("Intel", 1033);

    // Property
    builder.create_table("Property", vec![
        Column::build("Property").string(72).primary_key().build(),
        Column::build("Value").nullable().string(255).localizable().build(),
    ]).unwrap();
    builder.insert_rows("Property", vec![
        vec![Value::from("ProductName"), Value::from("File Table Test")],
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
        vec![Value::from("INSTALLDIR"), Value::from("ProgramFilesFolder"), Value::from("VelTest")],
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
        vec![Value::from("MainFeat"), Value::Null, Value::from("TestFeature"),
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

    // File table - standard 6 columns
    builder.create_table("File", vec![
        Column::build("File_").string(72).primary_key().build(),
        Column::build("Component_").string(72).build(),
        Column::build("FileName").string(255).localizable().build(),
        Column::build("FileSize").int32().build(),
        Column::build("Attributes").nullable().int16().build(),
        Column::build("Sequence").int16().build(),
    ]).unwrap();
    builder.insert_rows("File", vec![
        vec![Value::from("MainFile"), Value::from("MainComp"),
             Value::from("testfile.txt"), Value::Int(23),
             Value::Int(0), Value::Int(1)],
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

    let msi_data = builder.build().unwrap();

    let _ = std::fs::create_dir_all("C:\\temp");
    let path = "C:\\temp\\file_test.msi";
    let log_path = "C:\\temp\\file_test.log";
    let _ = std::fs::remove_file(path);
    let _ = std::fs::remove_file(log_path);
    std::fs::write(path, &msi_data).unwrap();

    println!("Testing File table MSI...");
    let output = std::process::Command::new("msiexec")
        .args(&["/i", path, "/qn", "/l*v", log_path])
        .output().unwrap();
    let ec = output.status.code().unwrap_or(-1);
    println!("Exit code: {}", ec);

    // Print FULL log
    if let Ok(log) = std::fs::read_to_string(log_path) {
        println!("\n=== FULL LOG ===");
        for line in log.lines() {
            // Print lines with errors, warnings, or important info
            if line.contains("error") || line.contains("Error") ||
               line.contains("return value 3") || line.contains("Note:") ||
               line.contains("MSI (s)") || line.contains("CustomAction") ||
               line.contains("File") || line.contains("Cost") ||
               line.contains("Component") {
                println!("{}", line.trim());
            }
        }
    }
}
