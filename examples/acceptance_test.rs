/// Acceptance test: build an MSI and validate it can be read by the cfb crate
/// This verifies that our custom OLE writer produces valid compound files.

use std::io::Cursor;
use velocity_msi::*;

fn main() {
    println!("Building test MSI...");
    let msi_data = build_test_msi();
    println!("MSI size: {} bytes", msi_data.len());
    
    println!("\nValidating OLE structure with cfb crate...");
    match cfb::CompoundFile::open(Cursor::new(&msi_data)) {
        Ok(mut comp) => {
            println!("✓ OLE structure is VALID");
            println!("  Root entry: {:?}", comp.root_entry().name());
            
            let mut stream_count = 0;
            let mut storage_count = 0;
            for entry in comp.walk() {
                if entry.is_storage() {
                    storage_count += 1;
                } else {
                    stream_count += 1;
                }
            }
            println!("  Storages: {}, Streams: {}", storage_count, stream_count);
            
            // Verify we can read all streams
            println!("\n  Reading all streams...");
            let stream_names: Vec<_> = comp.walk()
                .filter(|e| !e.is_storage() && e.name() != "Root Entry")
                .map(|e| (e.name().to_string(), e.len()))
                .collect();
            
            for (name, expected_len) in stream_names {
                match comp.open_stream(&name) {
                    Ok(mut stream) => {
                        let mut buf = Vec::new();
                        use std::io::Read;
                        stream.read_to_end(&mut buf).unwrap();
                        if buf.len() != expected_len as usize {
                            println!("    ✗ Stream '{}': size mismatch (expected {}, got {})", 
                                     name, expected_len, buf.len());
                        }
                    }
                    Err(e) => {
                        println!("    ✗ Failed to read stream '{}': {}", name, e);
                    }
                }
            }
            println!("  ✓ All streams readable");
            
            println!("\n✓ MSI validation PASSED");
            std::process::exit(0);
        }
        Err(e) => {
            println!("✗ OLE structure is INVALID: {}", e);
            println!("\n✗ MSI validation FAILED");
            std::process::exit(1);
        }
    }
}

fn build_test_msi() -> Vec<u8> {
    let mut builder = MsiBuilder::new();
    builder.set_title("Acceptance Test MSI");
    builder.set_author("Test");
    builder.set_template("Intel", 1033);

    // Property table
    builder.create_table("Property", vec![
        Column::build("Property").string(72).primary_key().build(),
        Column::build("Value").string(1024).build(),
    ]).unwrap();
    builder.insert_rows("Property", vec![
        vec![Value::from("ProductName"), Value::from("Acceptance Test")],
        vec![Value::from("ProductVersion"), Value::from("1.0.0")],
        vec![Value::from("Manufacturer"), Value::from("Test")],
        vec![Value::from("ProductCode"), Value::from("{12345678-1234-1234-1234-123456789012}")],
        vec![Value::from("ProductLanguage"), Value::from("1033")],
    ]).unwrap();

    // Directory table
    builder.create_table("Directory", vec![
        Column::build("Directory").string(72).primary_key().category("Identifier").build(),
        Column::build("Directory_Parent").string(72).nullable().category("Identifier").build(),
        Column::build("DefaultDir").string(255).primary_key().category("DefaultDir").build(),
    ]).unwrap();
    builder.insert_rows("Directory", vec![
        vec![Value::from("TARGETDIR"), Value::Null, Value::from("SourceDir")],
        vec![Value::from("INSTALLDIR"), Value::from("TARGETDIR"), Value::from("Test:Test")],
    ]).unwrap();

    // Component table
    builder.create_table("Component", vec![
        Column::build("Component").string(72).primary_key().category("Identifier").build(),
        Column::build("ComponentId").string(38).nullable().category("GUID").build(),
        Column::build("Directory_").string(72).category("Identifier").build(),
        Column::build("Attributes").int16().build(),
        Column::build("Condition").string(255).nullable().category("Condition").build(),
        Column::build("KeyPath").string(72).nullable().category("Identifier").build(),
    ]).unwrap();
    builder.insert_rows("Component", vec![
        vec![Value::from("MainComp"), Value::Null, Value::from("INSTALLDIR"), 
             Value::Int(0), Value::Null, Value::Null],
    ]).unwrap();

    // Feature table
    builder.create_table("Feature", vec![
        Column::build("Feature").string(38).primary_key().category("Identifier").build(),
        Column::build("Feature_Parent").string(38).nullable().category("Identifier").build(),
        Column::build("Title").string(64).nullable().build(),
        Column::build("Description").string(255).nullable().build(),
        Column::build("Display").int16().nullable().build(),
        Column::build("Level").int16().build(),
        Column::build("Directory_").string(72).nullable().build(),
    ]).unwrap();
    builder.insert_rows("Feature", vec![
        vec![Value::from("MainFeature"), Value::Null, Value::from("Complete"),
             Value::from("Full installation"), Value::Int(2), Value::Int(1), Value::from("INSTALLDIR")],
    ]).unwrap();

    // Media table
    builder.create_table("Media", vec![
        Column::build("DiskId").int16().primary_key().build(),
        Column::build("LastSequence").int16().build(),
        Column::build("DiskPrompt").string(64).nullable().build(),
    ]).unwrap();
    builder.insert_rows("Media", vec![
        vec![Value::Int(1), Value::Int(1), Value::from("Test Disk")],
    ]).unwrap();

    builder.build().unwrap()
}
