use velocity_msi::*;

fn main() {
    // Ultra-simple: 2 tables, 1 column each
    let mut b = MsiBuilder::new();
    b.set_title("Ultra simple");

    b.create_table("Alpha", vec![
        Column::build("Key").string(64).primary_key().build(),
    ]).unwrap();
    b.insert_rows("Alpha", vec![
        vec![Value::from("A1")],
    ]).unwrap();

    b.create_table("Beta", vec![
        Column::build("Key").string(64).primary_key().build(),
    ]).unwrap();
    b.insert_rows("Beta", vec![
        vec![Value::from("B1")],
    ]).unwrap();

    let data = b.build().unwrap();
    std::fs::write("test_ultra.msi", &data).unwrap();
    println!("Ultra-simple: {} bytes", data.len());
}
