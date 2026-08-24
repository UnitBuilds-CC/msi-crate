/// Test: Property + Directory + InstallExecuteSequence after removing
/// _Validation entries for _Tables/_Columns.
/// Previously failed with error 2705 ("Directory table could not be linked as tree").
use std::process::Command;

fn main() {
    println!("=== VALIDATION FIX TEST ===\n");

    let _ = Command::new("taskkill").args(&["/F", "/IM", "msiexec.exe"]).output();
    std::thread::sleep(std::time::Duration::from_secs(1));

    let mut b = velocity_msi::MsiBuilder::new();
    b.set_title("ValFixTest");
    b.set_author("Velocity");
    b.set_template("Intel", 1033);

    // Property table
    b.create_table("Property", vec![
        velocity_msi::Column::build("Property").string(72).primary_key().build(),
        velocity_msi::Column::build("Value").string(255).nullable().build(),
    ]).unwrap();
    b.insert_rows("Property", vec![
        vec![velocity_msi::Value::from("ProductName"), velocity_msi::Value::from("ValFixTest")],
        vec![velocity_msi::Value::from("ProductVersion"), velocity_msi::Value::from("1.0.0")],
        vec![velocity_msi::Value::from("Manufacturer"), velocity_msi::Value::from("V")],
        vec![velocity_msi::Value::from("ProductCode"), velocity_msi::Value::from("{AADDD4E4-3A12-4B7D-A4E0-5A2F8B3C6D90}")],
        vec![velocity_msi::Value::from("UpgradeCode"), velocity_msi::Value::from("{BBDDE5F5-4B23-5C8E-B5F1-6B3A9C4D7EA1}")],
        vec![velocity_msi::Value::from("ProductLanguage"), velocity_msi::Value::from("1033")],
    ]).unwrap();

    // Directory table
    b.create_table("Directory", vec![
        velocity_msi::Column::build("Directory").string(72).primary_key().build(),
        velocity_msi::Column::build("Directory_Parent").string(72).nullable().build(),
        velocity_msi::Column::build("DefaultDir").string(255).primary_key().build(),
    ]).unwrap();
    b.insert_rows("Directory", vec![
        vec![velocity_msi::Value::from("TARGETDIR"), velocity_msi::Value::Null, velocity_msi::Value::from("SourceDir")],
    ]).unwrap();

    // InstallExecuteSequence
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
    let path = "valfix_test.msi";
    std::fs::write(path, &msi_data).unwrap();
    println!("MSI: {} bytes", msi_data.len());

    // Test with msiexec
    let log_path = "valfix_test.log";
    let output = Command::new("msiexec")
        .args(&["/i", path, "/qn", "/norestart", "/l*v", log_path])
        .output()
        .expect("msiexec failed");
    let code = output.status.code().unwrap_or(-1);
    println!("Exit code: {}", code);

    match code {
        0 => {
            println!("SUCCESS! MSI installed!");
            // Uninstall
            let _ = Command::new("msiexec")
                .args(&["/x", path, "/qn", "/norestart"]).output();
        }
        1603 => {
            println!("1603 - Fatal error. Checking log...");
            if let Ok(log) = std::fs::read_to_string(log_path) {
                for line in log.lines() {
                    if line.contains("Error") || line.contains("2705") || line.contains("return value 3")
                        || line.contains("1620") || line.contains("return value 3")
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
