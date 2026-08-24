/// Test: InstallExecuteSequence with non-nullable Sequence column.
fn main() {
    let mut b = velocity_msi::MsiBuilder::new();
    b.set_title("ExecSeq2 Test");
    b.set_author("V");
    b.set_template("Intel", 1033);

    b.create_table("Property", vec![
        velocity_msi::Column::build("Property").string(72).primary_key().build(),
        velocity_msi::Column::build("Value").string(255).nullable().build(),
    ]).unwrap();
    b.insert_rows("Property", vec![
        vec![velocity_msi::Value::from("ProductName"), velocity_msi::Value::from("ExecSeq2")],
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

    // Try InstallExecuteSequence with NON-nullable Sequence (int16, not nullable)
    b.create_table("InstallExecuteSequence", vec![
        velocity_msi::Column::build("Action").string(72).primary_key().build(),
        velocity_msi::Column::build("Condition").string(255).nullable().build(),
        velocity_msi::Column::build("Sequence").int16().build(),  // NOT nullable
    ]).unwrap();
    b.insert_rows("InstallExecuteSequence", vec![
        vec![velocity_msi::Value::from("CostInitialize"), velocity_msi::Value::Null, velocity_msi::Value::Int(800)],
        vec![velocity_msi::Value::from("CostFinalize"), velocity_msi::Value::Null, velocity_msi::Value::Int(1000)],
    ]).unwrap();

    let data = b.build().unwrap();
    std::fs::write("execseq2_test.msi", &data).unwrap();
    println!("Wrote execseq2_test.msi ({} bytes)", data.len());
}
