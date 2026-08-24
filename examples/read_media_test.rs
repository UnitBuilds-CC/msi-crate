/// Test: use msi crate to open our MSI and read the Media table
/// cargo run --example read_media_test -p velocity-msi
use std::io::Cursor;
use velocity_msi::{Column, MsiBuilder, Value};

fn make_uuid() -> String {
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!(
        "{{{:08X}-{:04X}-4{:03X}-{:04X}-{:08X}{:04X}}}",
        (t & 0xFFFFFFFF) as u32,
        ((t >> 32) & 0xFFFF) as u16,
        ((t >> 48) & 0x0FFF) as u16,
        (((t >> 44) & 0x0FFF) as u16) | 0x8000,
        ((t >> 16) & 0xFFFFFFFF) as u32,
        (t & 0xFFFF) as u16,
    )
}

fn main() {
    println!("=== READ MEDIA TABLE TEST ===\n");

    let pc = make_uuid();
    let uc = make_uuid();

    let mut builder = MsiBuilder::new();
    builder.set_title("Read Media Test");
    builder.set_author("Velocity");
    builder.set_template("Intel", 1033);

    builder.create_table("Property", vec![
        Column::build("Property").string(72).primary_key().build(),
        Column::build("Value").nullable().string(255).localizable().build(),
    ]).unwrap();
    builder.insert_rows("Property", vec![
        vec![Value::from("ProductName"), Value::from("Read Media Test")],
        vec![Value::from("ProductVersion"), Value::from("1.0.0")],
        vec![Value::from("Manufacturer"), Value::from("Velocity")],
        vec![Value::from("ProductCode"), Value::from(pc.as_str())],
        vec![Value::from("UpgradeCode"), Value::from(uc.as_str())],
        vec![Value::from("ProductLanguage"), Value::from("1033")],
    ]).unwrap();

    builder.create_table("Media", vec![
        Column::build("DiskId").int16().primary_key().build(),
        Column::build("LastSequence").int16().build(),
        Column::build("DiskPrompt").nullable().string(64).localizable().build(),
        Column::build("Cabinet").nullable().string(255).build(),
        Column::build("VolumeLabel").nullable().string(32).localizable().build(),
        Column::build("Source").nullable().string(72).build(),
    ]).unwrap();
    builder.insert_rows("Media", vec![
        vec![Value::Int(1), Value::Int(1), Value::Null,
             Value::from("#velcab.cab"), Value::Null, Value::Null],
    ]).unwrap();

    let msi_data = builder.build().unwrap();
    println!("MSI built: {} bytes", msi_data.len());

    // Try to open with msi crate
    let cursor = Cursor::new(&msi_data);
    match msi::Package::open(cursor) {
        Ok(package) => {
            println!("msi crate opened MSI successfully!");
            
            // Try to read table names
            let tables = package.tables();
            println!("Table names:");
            for name in tables {
                println!("  {}", name.name());
            }
            
            // Try to read streams
            let streams: Vec<String> = package.streams().collect();
            println!("\nStreams:");
            for s in &streams {
                println!("  '{}'", s);
            }
        }
        Err(e) => {
            println!("msi crate FAILED to open MSI: {}", e);
            println!("This means our MSI structure is broken!");
        }
    }
}
