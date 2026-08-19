/// Test velocity-msi with a full set of MSI tables and msiexec.
use std::io::Cursor;
use velocity_msi::{Column, MsiBuilder as VelocityMsi, Value};

fn main() {
    std::fs::create_dir_all("C:\\temp").ok();

    let app_name = "Sample App";
    let app_version = "1.0.0";
    let publisher = "Test Corp";

    println!("Building MSI for: {} v{}", app_name, app_version);

    // Create MSI
    let mut builder = VelocityMsi::new();
    builder.set_title(app_name);
    builder.set_author(publisher);
    builder.set_template("Intel", 1033);
    builder.set_subject("Sample Application");

    // Create Property table with required properties
    builder.create_table("Property", vec![
        Column::build("Property").string(72).primary_key().build(),
        Column::build("Value").string(0).nullable().build(),
    ]).unwrap();

    let product_code = "{12345678-1234-1234-1234-123456789012}";
    let upgrade_code = "{87654321-4321-4321-4321-210987654321}";

    builder.insert_rows("Property", vec![
        vec![Value::from("ProductName"), Value::from(app_name)],
        vec![Value::from("ProductVersion"), Value::from(app_version)],
        vec![Value::from("Manufacturer"), Value::from(publisher)],
        vec![Value::from("ProductCode"), Value::from(product_code)],
        vec![Value::from("UpgradeCode"), Value::from(upgrade_code)],
        vec![Value::from("ProductLanguage"), Value::from("1033")],
    ]).unwrap();

    // Create Directory table
    builder.create_table("Directory", vec![
        Column::build("Directory").string(72).primary_key().build(),
        Column::build("Directory_Parent").string(72).nullable().build(),
        Column::build("DefaultDir").string(255).nullable().build(),
    ]).unwrap();
    builder.insert_rows("Directory", vec![
        vec![Value::from("TARGETDIR"), Value::Null, Value::from("SourceDir")],
        vec![Value::from("ProgramFilesFolder"), Value::from("TARGETDIR"), Value::from("PFiles")],
        vec![Value::from("INSTALLDIR"), Value::from("ProgramFilesFolder"), Value::from(app_name)],
    ]).unwrap();

    // Create Component table
    builder.create_table("Component", vec![
        Column::build("Component").string(72).primary_key().build(),
        Column::build("ComponentId").string(38).nullable().build(),
        Column::build("Directory_").string(72).build(),
        Column::build("Attributes").int16().build(),
        Column::build("Condition").string(255).nullable().build(),
        Column::build("KeyPath").string(72).nullable().build(),
    ]).unwrap();
    builder.insert_rows("Component", vec![
        vec![Value::from("MainComponent"), Value::Null, Value::from("INSTALLDIR"), Value::Int(0), Value::Null, Value::Null],
    ]).unwrap();

    // Create Feature table
    builder.create_table("Feature", vec![
        Column::build("Feature").string(38).primary_key().build(),
        Column::build("Feature_Parent").string(38).nullable().build(),
        Column::build("Title").string(64).nullable().build(),
        Column::build("Description").string(255).nullable().build(),
        Column::build("Display").int16().nullable().build(),
        Column::build("Level").int16().build(),
    ]).unwrap();
    builder.insert_rows("Feature", vec![
        vec![Value::from("Complete"), Value::Null, Value::from("Complete"), Value::from("Full installation"), Value::Int(2), Value::Int(1)],
    ]).unwrap();

    // Create FeatureComponents table
    builder.create_table("FeatureComponents", vec![
        Column::build("Feature_").string(38).primary_key().build(),
        Column::build("Component_").string(72).primary_key().build(),
    ]).unwrap();
    builder.insert_rows("FeatureComponents", vec![
        vec![Value::from("Complete"), Value::from("MainComponent")],
    ]).unwrap();

    // Create Media table
    builder.create_table("Media", vec![
        Column::build("DiskId").int16().primary_key().build(),
        Column::build("LastSequence").int32().build(),
        Column::build("DiskPrompt").string(64).nullable().build(),
        Column::build("Cabinet").string(255).nullable().build(),
        Column::build("VolumeLabel").string(32).nullable().build(),
        Column::build("Source").string(72).nullable().build(),
    ]).unwrap();
    builder.insert_rows("Media", vec![
        vec![Value::Int(1), Value::Int(0), Value::Null, Value::Null, Value::Null, Value::Null],
    ]).unwrap();

    // Build the MSI
    let msi_data = builder.build().unwrap();
    let path = "C:\\temp\\sample_app.msi";
    std::fs::write(path, &msi_data).unwrap();
    println!("\nGenerated MSI: {} ({} bytes)", path, msi_data.len());

    // Test with msiexec
    println!("\n=== msiexec test ===");
    let output = std::process::Command::new("msiexec.exe")
        .args(&["/i", path, "/quiet", "/norestart"])
        .output().unwrap();
    let code = output.status.code().unwrap_or(-1);
    let status = match code {
        0 => "SUCCESS (installed!)",
        1603 => "FATAL ERROR (during install)",
        1613 => "OK (opens, can't repair)",
        1620 => "FAIL (can't open)",
        1625 => "OK (opens, blocked by policy)",
        _ => "OTHER",
    };
    println!("msiexec exit code: {} ({})", code, status);

    // Also verify with msi crate that it can read our MSI
    println!("\n=== msi crate verification ===");
    match msi::Package::open(Cursor::new(msi_data)) {
        Ok(pkg) => {
            println!("msi crate can read our MSI!");
            let table_names: Vec<String> = pkg.tables().map(|t| t.name().to_string()).collect();
            println!("  Tables: {}", table_names.len());
            for name in &table_names {
                println!("  {}", name);
            }
        }
        Err(e) => println!("msi crate FAILED: {}", e),
    }
}
