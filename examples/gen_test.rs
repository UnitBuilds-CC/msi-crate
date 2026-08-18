use velocity_msi::{MsiBuilder, Column, Value};

fn main() {
    let mut builder = MsiBuilder::new();
    builder.set_title("Velocity Test App");
    builder.set_author("Velocity Installer");
    builder.set_subject("Test Installation");
    builder.set_template("Intel", 1033);

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
                vec![Value::from("ProductName"), Value::from("Velocity Test App")],
                vec![Value::from("ProductVersion"), Value::from("1.0.0")],
                vec![Value::from("Manufacturer"), Value::from("Velocity")],
                vec![Value::from("ProductCode"), Value::from("{12345678-1234-1234-1234-123456789012}")],
                vec![Value::from("UpgradeCode"), Value::from("{87654321-4321-4321-4321-210987654321}")],
            ],
        )
        .unwrap();

    let msi_data = builder.build().unwrap();
    std::fs::write("target/test_velocity.msi", &msi_data).unwrap();
    println!("Created target/test_velocity.msi ({} bytes)", msi_data.len());
}
