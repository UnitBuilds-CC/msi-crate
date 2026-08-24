/// Install test - fix Directory tree structure
use velocity_msi::{MsiBuilder, Column, Value, CabinetFile, build_cabinet};

fn main() {
    // Uninstall any previous test products
    for code in &["{BBBBBBBB-BBBB-BBBB-BBBB-BBBBBBBBBBBB}",
                  "{12345678-1234-1234-1234-123456789ABC}",
                  "{AAAAAAAA-AAAA-AAAA-AAAA-AAAAAAAAAAAA}"] {
        let _ = std::process::Command::new("msiexec")
            .args(&["/x", code, "/qn", "/norestart"]).output();
    }
    std::thread::sleep(std::time::Duration::from_secs(1));

    let mut builder = MsiBuilder::new();
    builder.set_title("Velocity Install Test");
    builder.set_author("Velocity");
    builder.set_template("x64", 1033);

    // Properties
    builder.create_table("Property", vec![
        Column::build("Property").string(72).primary_key().build(),
        Column::build("Value").string(255).nullable().build(),
    ]).unwrap();

    let product_code = "{CCCCCCCC-CCCC-CCCC-CCCC-CCCCCCCCCCCC}";
    let upgrade_code = "{87654321-4321-4321-4321-CBA987654321}";

    builder.insert_rows("Property", vec![
        vec![Value::from("ProductName"), Value::from("Velocity Test")],
        vec![Value::from("ProductVersion"), Value::from("1.0.0")],
        vec![Value::from("Manufacturer"), Value::from("Velocity Corp")],
        vec![Value::from("ProductCode"), Value::from(product_code)],
        vec![Value::from("UpgradeCode"), Value::from(upgrade_code)],
        vec![Value::from("ProductLanguage"), Value::from("1033")],
    ]).unwrap();

    // Directory structure - use proper MSI DefaultDir format
    // ShortName:LongName format. When no long name, just use Name.
    builder.create_table("Directory", vec![
        Column::build("Directory").string(72).primary_key().build(),
        Column::build("Directory_Parent").string(72).nullable().build(),
        Column::build("DefaultDir").string(255).primary_key().build(),
    ]).unwrap();
    builder.insert_rows("Directory", vec![
        vec![Value::from("TARGETDIR"), Value::Null, Value::from("SourceDir")],
        vec![Value::from("INSTALLDIR"), Value::from("TARGETDIR"), Value::from("VelTest:VelTest")],
    ]).unwrap();

    // Component
    builder.create_table("Component", vec![
        Column::build("Component").string(72).primary_key().build(),
        Column::build("ComponentId").string(38).nullable().build(),
        Column::build("Directory_").string(72).build(),
        Column::build("Attributes").int16().build(),
        Column::build("Condition").string(255).nullable().build(),
        Column::build("KeyPath").string(72).nullable().build(),
    ]).unwrap();
    builder.insert_rows("Component", vec![
        vec![Value::from("Comp1"), Value::Null, Value::from("INSTALLDIR"),
             Value::Int(0), Value::Null, Value::from("hello.txt")],
    ]).unwrap();

    // File table - 8 columns per MSI spec
    builder.create_table("File", vec![
        Column::build("File").string(72).primary_key().build(),
        Column::build("Component_").string(72).build(),
        Column::build("FileName").string(255).build(),
        Column::build("FileSize").int32().build(),
        Column::build("Sequence").int16().build(),
        Column::build("Version").string(72).nullable().build(),
        Column::build("Language").string(20).nullable().build(),
        Column::build("Attributes").int16().nullable().build(),
    ]).unwrap();
    let content = b"Hello World from Velocity Installer!\r\n";
    builder.insert_rows("File", vec![
        vec![Value::from("hello.txt"), Value::from("Comp1"), Value::from("hello.txt"),
             Value::Int(content.len() as i32), Value::Int(1),
             Value::Null, Value::Null, Value::Null],
    ]).unwrap();

    // Feature
    builder.create_table("Feature", vec![
        Column::build("Feature").string(38).primary_key().build(),
        Column::build("Feature_Parent").string(38).nullable().build(),
        Column::build("Title").string(64).nullable().build(),
        Column::build("Description").string(255).nullable().build(),
        Column::build("Display").int16().nullable().build(),
        Column::build("Level").int16().build(),
        Column::build("Directory_").string(72).nullable().build(),
        Column::build("Attributes").int16().nullable().build(),
    ]).unwrap();
    builder.insert_rows("Feature", vec![
        vec![Value::from("Feat1"), Value::Null, Value::from("Main"),
             Value::from("Main feature"), Value::Int(1), Value::Int(1),
             Value::Null, Value::Null],
    ]).unwrap();

    // FeatureComponents
    builder.create_table("FeatureComponents", vec![
        Column::build("Feature_").string(38).primary_key().build(),
        Column::build("Component_").string(72).primary_key().build(),
    ]).unwrap();
    builder.insert_rows("FeatureComponents", vec![
        vec![Value::from("Feat1"), Value::from("Comp1")],
    ]).unwrap();

    // Media table with embedded cabinet
    builder.create_table("Media", vec![
        Column::build("DiskId").int16().primary_key().build(),
        Column::build("LastSequence").int16().build(),
        Column::build("DiskPrompt").string(64).nullable().build(),
        Column::build("Cabinet").string(255).nullable().build(),
        Column::build("VolumeLabel").string(32).nullable().build(),
        Column::build("Source").string(72).nullable().build(),
    ]).unwrap();

    let cab = build_cabinet(&[CabinetFile {
        name: "hello.txt".to_string(),
        data: content.to_vec(),
    }]);
    builder.insert_rows("Media", vec![
        vec![Value::Int(1), Value::Int(1), Value::Null,
             Value::from("#vel.cab"), Value::Null, Value::Null],
    ]).unwrap();
    builder.add_stream("vel.cab".to_string(), cab);

    // InstallExecuteSequence - required for msiexec to run install actions
    builder.create_table("InstallExecuteSequence", vec![
        Column::build("Action").string(72).primary_key().build(),
        Column::build("Condition").string(255).nullable().build(),
        Column::build("Sequence").int16().nullable().build(),
    ]).unwrap();
    builder.insert_rows("InstallExecuteSequence", vec![
        vec![Value::from("LaunchConditions"), Value::Null, Value::Int(100)],
        vec![Value::from("CostInitialize"), Value::Null, Value::Int(800)],
        vec![Value::from("FileCost"), Value::Null, Value::Int(900)],
        vec![Value::from("CostFinalize"), Value::Null, Value::Int(1000)],
        vec![Value::from("InstallValidate"), Value::Null, Value::Int(1400)],
        vec![Value::from("InstallInitialize"), Value::Null, Value::Int(1500)],
        vec![Value::from("ProcessComponents"), Value::Null, Value::Int(1600)],
        vec![Value::from("InstallFiles"), Value::Null, Value::Int(4000)],
        vec![Value::from("RegisterProduct"), Value::Null, Value::Int(6100)],
        vec![Value::from("PublishProduct"), Value::Null, Value::Int(6200)],
        vec![Value::from("InstallFinalize"), Value::Null, Value::Int(6600)],
    ]).unwrap();

    // InstallUISequence
    builder.create_table("InstallUISequence", vec![
        Column::build("Action").string(72).primary_key().build(),
        Column::build("Condition").string(255).nullable().build(),
        Column::build("Sequence").int16().nullable().build(),
    ]).unwrap();
    builder.insert_rows("InstallUISequence", vec![
        vec![Value::from("LaunchConditions"), Value::Null, Value::Int(100)],
        vec![Value::from("CostInitialize"), Value::Null, Value::Int(800)],
        vec![Value::from("FileCost"), Value::Null, Value::Int(900)],
        vec![Value::from("CostFinalize"), Value::Null, Value::Int(1000)],
        vec![Value::from("ExecuteAction"), Value::Null, Value::Int(1300)],
    ]).unwrap();

    let msi = builder.build().unwrap();
    std::fs::write("test_install.msi", &msi).unwrap();
    println!("MSI: {} bytes", msi.len());

    let output = std::process::Command::new("msiexec")
        .args(&["/i", "test_install.msi", "/qn", "/l*v", "test_install_log.txt"])
        .output().unwrap();
    let exit_code = output.status.code().unwrap_or(-1);
    println!("Exit code: {}", exit_code);

    // Check if file was installed
    for dir in &[r"C:\VelTest", r"C:\Program Files\VelTest", r"C:\Program Files (x86)\VelTest"] {
        if std::path::Path::new(dir).exists() {
            println!("Directory {} exists:", dir);
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries {
                    let entry = entry.unwrap();
                    println!("  {}", entry.path().to_string_lossy());
                }
            }
        }
    }

    // Print key log lines
    if let Ok(log) = std::fs::read_to_string("test_install_log.txt") {
        println!("\n--- Key log entries ---");
        for line in log.lines() {
            if line.contains("Action start") || line.contains("Action ended")
                || (line.contains("Error") && !line.contains("Error 0") && !line.contains("Error 0."))
                || line.contains("cabinet") || line.contains("Cabinet")
                || line.contains("2705") || line.contains("2725") || line.contains("1603")
                || line.contains("Return value")
            {
                println!("  {}", line.trim());
            }
        }
    }
}
