use velocity_msi::{Column, MsiBuilder, Value};

fn main() {
    let mut builder = MsiBuilder::new();
    builder.set_title("Test MSI");
    builder.set_author("Velocity Installer");
    builder.set_template("x64", 1033);

    // Create Property table
    builder
        .create_table(
            "Property",
            vec![
                Column::build("Property").string(72).primary_key().build(),
                Column::build("Value").string(255).nullable().build(),
            ],
        )
        .unwrap();

    // Insert some properties
    builder
        .insert_rows(
            "Property",
            vec![
                vec![Value::from("ProductName"), Value::from("Test Product")],
                vec![Value::from("ProductVersion"), Value::from("1.0.0")],
                vec![Value::from("Manufacturer"), Value::from("Test Company")],
            ],
        )
        .unwrap();

    // Build the MSI
    let msi_data = builder.build().unwrap();

    // Write to file
    std::fs::create_dir_all("target").ok();
    std::fs::write("target/test_velocity_msi.msi", &msi_data).unwrap();
    println!(
        "Created MSI: {} bytes at target/test_velocity_msi.msi",
        msi_data.len()
    );
}
