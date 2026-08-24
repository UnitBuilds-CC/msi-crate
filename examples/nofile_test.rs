/// Test: MSI without files (no cabinet) - should install successfully
use velocity_msi::{MsiBuilder, Column, Value};

fn main() {
    let mut builder = MsiBuilder::new();
    builder.set_title("NoFile Test");
    builder.set_author("Velocity");
    builder.set_template("x64", 1033);

    // Property table
    builder.create_table("Property", vec![
        Column::build("Property").string(72).primary_key().build(),
        Column::build("Value").string(255).nullable().build(),
    ]).unwrap();

    let product_code = "{AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE}";
    let upgrade_code = "{11111111-2222-3333-4444-555555555555}";

    builder.insert_rows("Property", vec![
        vec![Value::from("ProductName"), Value::from("NoFile Test")],
        vec![Value::from("ProductVersion"), Value::from("1.0.0")],
        vec![Value::from("Manufacturer"), Value::from("Velocity Corp")],
        vec![Value::from("ProductCode"), Value::from(product_code)],
        vec![Value::from("UpgradeCode"), Value::from(upgrade_code)],
        vec![Value::from("ProductLanguage"), Value::from("1033")],
    ]).unwrap();

    // Directory table
    builder.create_table("Directory", vec![
        Column::build("Directory").string(72).primary_key().build(),
        Column::build("Directory_Parent").string(72).nullable().build(),
        Column::build("DefaultDir").string(255).primary_key().build(),
    ]).unwrap();
    builder.insert_rows("Directory", vec![
        vec![Value::from("TARGETDIR"), Value::Null, Value::from("SourceDir")],
    ]).unwrap();

    // Component table (one component with no files)
    builder.create_table("Component", vec![
        Column::build("Component").string(72).primary_key().build(),
        Column::build("ComponentId").string(38).nullable().build(),
        Column::build("Directory_").string(72).build(),
        Column::build("Attributes").int16().build(),
        Column::build("Condition").string(255).nullable().build(),
        Column::build("KeyPath").string(72).nullable().build(),
    ]).unwrap();
    builder.insert_rows("Component", vec![
        vec![Value::from("Comp1"), Value::Null, Value::from("TARGETDIR"),
             Value::Int(2), Value::Null, Value::Null],  // Attrs=2 (no file, keypath=directory)
    ]).unwrap();

    // Feature table
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

    // NO Media table (no files to install)

    let msi = builder.build().unwrap();
    std::fs::write("nofile_test.msi", &msi).unwrap();
    println!("MSI: {} bytes", msi.len());

    // Install
    let output = std::process::Command::new("msiexec")
        .args(&["/i", "nofile_test.msi", "/qn", "/l*v", "nofile_test_log.txt"])
        .output().unwrap();
    let code = output.status.code().unwrap_or(-1);
    println!("Install exit: {}", code);

    // Check log
    if let Ok(log) = std::fs::read_to_string("nofile_test_log.txt") {
        for line in log.lines() {
            if line.contains("returning") || line.contains("Note: 1:") 
                || line.contains("Error") && !line.contains("Error 0")
                || line.contains("Product:") {
                println!("  {}", line.trim());
            }
        }
    }

    // Uninstall if install succeeded
    if code == 0 {
        println!("\n=== Uninstall ===");
        let output = std::process::Command::new("msiexec")
            .args(&["/x", product_code, "/qn", "/l*v", "nofile_uninstall_log.txt"])
            .output().unwrap();
        println!("Uninstall exit: {}", output.status.code().unwrap_or(-1));
    }

    // Cleanup
    let _ = std::fs::remove_file("nofile_test.msi");
    let _ = std::fs::remove_file("nofile_test_log.txt");
    let _ = std::fs::remove_file("nofile_uninstall_log.txt");
}
