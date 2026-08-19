use velocity_msi::*;

fn main() {
    // Test with simple table names
    let mut b = MsiBuilder::new();
    b.set_title("Test MSI simple names");
    b.set_template("x64", 1033);

    b.create_table("Table1", vec![
        Column::build("Col1").string(64).primary_key().build(),
        Column::build("Col2").string(255).nullable().build(),
    ]).unwrap();
    b.insert_rows("Table1", vec![
        vec![Value::from("Key1"), Value::from("Value1")],
    ]).unwrap();

    b.create_table("Table2", vec![
        Column::build("ColA").string(64).primary_key().build(),
        Column::build("ColB").string(255).nullable().build(),
    ]).unwrap();
    b.insert_rows("Table2", vec![
        vec![Value::from("KeyA"), Value::from("ValueA")],
    ]).unwrap();

    let data = b.build().unwrap();
    std::fs::write("test_simple.msi", &data).unwrap();
    println!("Simple names: {} bytes", data.len());
}
