/// Test: Directory table only (no InstallExecuteSequence) to isolate error 2705.
fn main() {
    println!("=== Directory-only MSI test ===\n");

    let mut b = velocity_msi::MsiBuilder::new();
    b.set_title("Dir Only Test");
    b.set_author("Velocity");
    b.set_template("Intel", 1033);

    b.create_table("Property", vec![
        velocity_msi::Column::build("Property").string(72).primary_key().build(),
        velocity_msi::Column::build("Value").string(255).nullable().build(),
    ]).unwrap();
    b.insert_rows("Property", vec![
        vec![velocity_msi::Value::from("ProductName"), velocity_msi::Value::from("Dir Test")],
        vec![velocity_msi::Value::from("ProductVersion"), velocity_msi::Value::from("1.0.0")],
        vec![velocity_msi::Value::from("Manufacturer"), velocity_msi::Value::from("Velocity")],
        vec![velocity_msi::Value::from("ProductCode"), velocity_msi::Value::from("{AADDD4E4-3A12-4B7D-A4E0-5A2F8B3C6D90}")],
        vec![velocity_msi::Value::from("UpgradeCode"), velocity_msi::Value::from("{BBDDE5F5-4B23-5C8E-B5F1-6B3A9C4D7EA1}")],
        vec![velocity_msi::Value::from("ProductLanguage"), velocity_msi::Value::from("1033")],
    ]).unwrap();

    // Simple directory: just TARGETDIR with no children
    b.create_table("Directory", vec![
        velocity_msi::Column::build("Directory").string(72).primary_key().build(),
        velocity_msi::Column::build("Directory_Parent").string(72).nullable().build(),
        velocity_msi::Column::build("DefaultDir").string(255).primary_key().build(),
    ]).unwrap();
    b.insert_rows("Directory", vec![
        vec![velocity_msi::Value::from("TARGETDIR"), velocity_msi::Value::Null, velocity_msi::Value::from("SourceDir")],
    ]).unwrap();

    let msi_data = b.build().unwrap();
    let path = "dir_only_test.msi";
    std::fs::write(path, &msi_data).unwrap();
    println!("Wrote {} ({} bytes)", path, msi_data.len());

    // Verify with msi crate
    let file = std::fs::File::open(path).unwrap();
    match msi::Package::open(file) {
        Ok(pkg) => {
            println!("msi crate open: OK");
            for table in pkg.tables() {
                println!("  Table '{}': {} cols", table.name(),
                    table.columns().len());
            }
        }
        Err(e) => println!("msi crate open FAILED: {}", e),
    }
    println!("\nNow test with: msiexec /i {} /qn /l*v dir_only.log", path);
}
