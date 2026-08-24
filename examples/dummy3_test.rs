/// Test: does ANY third table cause error 2705, or just InstallExecuteSequence?
fn main() {
    // Test 1: Dummy table (no standard action processing)
    let mut b = velocity_msi::MsiBuilder::new();
    b.set_title("Dummy3 Test");
    b.set_author("V");
    b.set_template("Intel", 1033);
    b.create_table("Property", vec![
        velocity_msi::Column::build("Property").string(72).primary_key().build(),
        velocity_msi::Column::build("Value").string(255).nullable().build(),
    ]).unwrap();
    b.insert_rows("Property", vec![
        vec![velocity_msi::Value::from("ProductName"), velocity_msi::Value::from("Dummy3")],
        vec![velocity_msi::Value::from("ProductVersion"), velocity_msi::Value::from("1.0.0")],
        vec![velocity_msi::Value::from("Manufacturer"), velocity_msi::Value::from("V")],
        vec![velocity_msi::Value::from("ProductCode"), velocity_msi::Value::from("{AADDD4E4-3A12-4B7D-A4E0-5A2F8B3C6D90}")],
        vec![velocity_msi::Value::from("UpgradeCode"), velocity_msi::Value::from("{BBDDE5F5-4B23-5C8E-B5F1-6B3A9C4D7EA1}")],
        vec![velocity_msi::Value::from("ProductLanguage"), velocity_msi::Value::from("1033")],
    ]).unwrap();
    b.create_table("Directory", vec![
        velocity_msi::Column::build("Directory").string(72).primary_key().build(),
        velocity_msi::Column::build("Directory_Parent").string(72).nullable().build(),
        velocity_msi::Column::build("DefaultDir").string(255).primary_key().build(),
    ]).unwrap();
    b.insert_rows("Directory", vec![
        vec![velocity_msi::Value::from("TARGETDIR"), velocity_msi::Value::Null, velocity_msi::Value::from("SourceDir")],
        vec![velocity_msi::Value::from("INSTALLDIR"), velocity_msi::Value::from("TARGETDIR"), velocity_msi::Value::from("VelTest")],
    ]).unwrap();
    // Dummy third table
    b.create_table("ZZZ_Dummy", vec![
        velocity_msi::Column::build("Key").string(72).primary_key().build(),
        velocity_msi::Column::build("Val").string(255).nullable().build(),
    ]).unwrap();
    b.insert_rows("ZZZ_Dummy", vec![
        vec![velocity_msi::Value::from("test"), velocity_msi::Value::from("value")],
    ]).unwrap();

    let data = b.build().unwrap();
    std::fs::write("dummy3_test.msi", &data).unwrap();
    println!("Wrote dummy3_test.msi ({} bytes)", data.len());
}
