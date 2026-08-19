use velocity_msi::*;

fn main() {
    // Test 1: Multiple tables, no extra streams
    let mut b = MsiBuilder::new();
    b.set_title("Test MSI multi-table");
    b.set_template("x64", 1033);

    b.create_table("Property", vec![
        Column::build("Property").string(64).primary_key().build(),
        Column::build("Value").string(255).nullable().build(),
    ]).unwrap();
    b.insert_rows("Property", vec![
        vec![Value::from("ProductName"), Value::from("Test")],
    ]).unwrap();

    b.create_table("Directory", vec![
        Column::build("Directory").string(64).primary_key().build(),
        Column::build("Directory_Parent").string(64).nullable().build(),
        Column::build("DefaultDir").string(255).nullable().build(),
    ]).unwrap();
    b.insert_rows("Directory", vec![
        vec![Value::from("TARGETDIR"), Value::Null, Value::from(".")],
    ]).unwrap();

    let data = b.build().unwrap();
    std::fs::write("test_multi.msi", &data).unwrap();
    println!("Multi-table (no extra streams): {} bytes", data.len());

    // Test 2: Single table + extra stream
    let mut b2 = MsiBuilder::new();
    b2.set_title("Test MSI with stream");
    b2.set_template("x64", 1033);

    b2.create_table("Property", vec![
        Column::build("Property").string(64).primary_key().build(),
        Column::build("Value").string(255).nullable().build(),
    ]).unwrap();
    b2.insert_rows("Property", vec![
        vec![Value::from("ProductName"), Value::from("Test")],
    ]).unwrap();

    b2.add_stream("TestCab.dat".to_string(), vec![0u8; 5000]);

    let data2 = b2.build().unwrap();
    std::fs::write("test_stream.msi", &data2).unwrap();
    println!("Single table + extra stream: {} bytes", data2.len());
}
