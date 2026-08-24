/// Test: Does CostInitialize need Feature/Component/Media tables?
/// If adding them fixes 2705, the issue is missing required tables.
use std::process::Command;

fn main() {
    println!("=== FEATURE TABLE TEST ===\n");

    let _ = Command::new("taskkill").args(&["/F", "/IM", "msiexec.exe"]).output();
    std::thread::sleep(std::time::Duration::from_secs(1));

    let mut b = velocity_msi::MsiBuilder::new();
    b.set_title("FeatureTest");
    b.set_author("Velocity");
    b.set_template("Intel", 1033);

    // Property table
    b.create_table("Property", vec![
        velocity_msi::Column::build("Property").string(72).primary_key().build(),
        velocity_msi::Column::build("Value").string(255).nullable().build(),
    ]).unwrap();
    b.insert_rows("Property", vec![
        vec![velocity_msi::Value::from("ProductName"), velocity_msi::Value::from("FeatureTest")],
        vec![velocity_msi::Value::from("ProductVersion"), velocity_msi::Value::from("1.0.0")],
        vec![velocity_msi::Value::from("Manufacturer"), velocity_msi::Value::from("V")],
        vec![velocity_msi::Value::from("ProductCode"), velocity_msi::Value::from("{AADDD4E4-3A12-4B7D-A4E0-5A2F8B3C6D90}")],
        vec![velocity_msi::Value::from("UpgradeCode"), velocity_msi::Value::from("{BBDDE5F5-4B23-5C8E-B5F1-6B3A9C4D7EA1}")],
        vec![velocity_msi::Value::from("ProductLanguage"), velocity_msi::Value::from("1033")],
    ]).unwrap();

    // Directory with proper tree (3 levels)
    b.create_table("Directory", vec![
        velocity_msi::Column::build("Directory").string(72).primary_key().build(),
        velocity_msi::Column::build("Directory_Parent").string(72).nullable().build(),
        velocity_msi::Column::build("DefaultDir").string(255).primary_key().build(),
    ]).unwrap();
    b.insert_rows("Directory", vec![
        vec![velocity_msi::Value::from("TARGETDIR"), velocity_msi::Value::Null, velocity_msi::Value::from("SourceDir")],
        vec![velocity_msi::Value::from("ProgramFilesFolder"), velocity_msi::Value::from("TARGETDIR"), velocity_msi::Value::from("PFiles")],
        vec![velocity_msi::Value::from("INSTALLDIR"), velocity_msi::Value::from("ProgramFilesFolder"), velocity_msi::Value::from("FeatureTest")],
    ]).unwrap();

    // Feature table
    b.create_table("Feature", vec![
        velocity_msi::Column::build("Feature").string(38).primary_key().build(),
        velocity_msi::Column::build("Feature_Parent").string(38).nullable().build(),
        velocity_msi::Column::build("Title").string(64).nullable().build(),
        velocity_msi::Column::build("Description").string(255).nullable().build(),
        velocity_msi::Column::build("Display").int16().nullable().build(),
        velocity_msi::Column::build("Level").int16().build(),
        velocity_msi::Column::build("Directory_").string(72).nullable().build(),
        velocity_msi::Column::build("Attributes").int16().nullable().build(),
    ]).unwrap();
    b.insert_rows("Feature", vec![
        vec![
            velocity_msi::Value::from("Complete"),
            velocity_msi::Value::Null,
            velocity_msi::Value::from("Complete"),
            velocity_msi::Value::Null,
            velocity_msi::Value::Int(0),
            velocity_msi::Value::Int(1),
            velocity_msi::Value::from("INSTALLDIR"),
            velocity_msi::Value::Null,
        ],
    ]).unwrap();

    // Component table
    b.create_table("Component", vec![
        velocity_msi::Column::build("Component").string(72).primary_key().build(),
        velocity_msi::Column::build("ComponentId").string(38).nullable().build(),
        velocity_msi::Column::build("Directory_").string(72).build(),
        velocity_msi::Column::build("Attributes").int16().nullable().build(),
        velocity_msi::Column::build("Condition").string(255).nullable().build(),
        velocity_msi::Column::build("KeyPath").string(72).nullable().build(),
    ]).unwrap();
    b.insert_rows("Component", vec![
        vec![
            velocity_msi::Value::from("MainComponent"),
            velocity_msi::Value::Null,
            velocity_msi::Value::from("INSTALLDIR"),
            velocity_msi::Value::Null,
            velocity_msi::Value::Null,
            velocity_msi::Value::from("MainComponent"),
        ],
    ]).unwrap();

    // Media table
    b.create_table("Media", vec![
        velocity_msi::Column::build("DiskId").int16().primary_key().build(),
        velocity_msi::Column::build("LastSequence").int16().build(),
        velocity_msi::Column::build("VolumeLabel").string(32).nullable().build(),
        velocity_msi::Column::build("Cabinet").string(255).nullable().build(),
        velocity_msi::Column::build("MediaSrcPrompt").string(64).nullable().build(),
    ]).unwrap();
    b.insert_rows("Media", vec![
        vec![
            velocity_msi::Value::Int(1),
            velocity_msi::Value::Int(0),
            velocity_msi::Value::Null,
            velocity_msi::Value::Null,
            velocity_msi::Value::Null,
        ],
    ]).unwrap();

    // InstallExecuteSequence with CostInitialize
    b.create_table("InstallExecuteSequence", vec![
        velocity_msi::Column::build("Action").string(72).primary_key().build(),
        velocity_msi::Column::build("Condition").string(255).nullable().build(),
        velocity_msi::Column::build("Sequence").int16().nullable().build(),
    ]).unwrap();
    b.insert_rows("InstallExecuteSequence", vec![
        vec![velocity_msi::Value::from("CostInitialize"), velocity_msi::Value::Null, velocity_msi::Value::Int(800)],
        vec![velocity_msi::Value::from("CostFinalize"), velocity_msi::Value::Null, velocity_msi::Value::Int(1000)],
    ]).unwrap();

    let msi_data = b.build().unwrap();
    let path = "feature_test.msi";
    std::fs::write(path, &msi_data).unwrap();
    println!("MSI: {} bytes", msi_data.len());

    // Test with msiexec
    let log_path = "feature_test.log";
    let output = Command::new("msiexec")
        .args(&["/i", path, "/qn", "/norestart", "/l*v", log_path])
        .output()
        .expect("msiexec failed");
    let code = output.status.code().unwrap_or(-1);
    println!("Exit code: {}", code);

    match code {
        0 => {
            println!("SUCCESS!");
            let _ = Command::new("msiexec")
                .args(&["/x", path, "/qn", "/norestart"]).output();
        }
        1603 => {
            println!("1603 - Fatal error. Checking log...");
            if let Ok(log) = std::fs::read_to_string(log_path) {
                for line in log.lines() {
                    if line.contains("Error") || line.contains("2705") || line.contains("return value 3")
                        || line.contains("1620")
                    {
                        println!("  {}", line.trim());
                    }
                }
            }
        }
        1620 => println!("1620 - Could not open package"),
        _ => println!("Error code: {}", code),
    }
}
