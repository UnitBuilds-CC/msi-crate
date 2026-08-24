/// Use cfb to repackage: take our nofile MSI (which works) and add cabinet + Media table
/// This isolates whether the issue is OLE structure or MSI data.
use std::io::{Read, Write, Cursor, Seek, SeekFrom};
use velocity_msi::{MsiBuilder, Column, Value, CabinetFile, build_cabinet};

fn main() {
    // Step 1: Build a working MSI (no files, no cabinet)
    let mut builder = MsiBuilder::new();
    builder.set_title("CFB Test");
    builder.set_author("Velocity");
    builder.set_template("x64", 1033);

    builder.create_table("Property", vec![
        Column::build("Property").string(72).primary_key().build(),
        Column::build("Value").string(255).nullable().build(),
    ]).unwrap();

    let product_code = "{BBBBBBBB-CCCC-DDDD-EEEE-FFFFFFFFFFFF}";
    let upgrade_code = "{11111111-2222-3333-4444-555555555555}";

    builder.insert_rows("Property", vec![
        vec![Value::from("ProductName"), Value::from("CFB Test")],
        vec![Value::from("ProductVersion"), Value::from("1.0.0")],
        vec![Value::from("Manufacturer"), Value::from("Velocity Corp")],
        vec![Value::from("ProductCode"), Value::from(product_code)],
        vec![Value::from("UpgradeCode"), Value::from(upgrade_code)],
        vec![Value::from("ProductLanguage"), Value::from("1033")],
    ]).unwrap();

    builder.create_table("Directory", vec![
        Column::build("Directory").string(72).primary_key().build(),
        Column::build("Directory_Parent").string(72).nullable().build(),
        Column::build("DefaultDir").string(255).primary_key().build(),
    ]).unwrap();
    builder.insert_rows("Directory", vec![
        vec![Value::from("TARGETDIR"), Value::Null, Value::from("SourceDir")],
        vec![Value::from("INSTALLDIR"), Value::from("TARGETDIR"), Value::from("CFBTest:.")],
    ]).unwrap();

    builder.create_table("Component", vec![
        Column::build("Component").string(72).primary_key().build(),
        Column::build("ComponentId").string(38).nullable().build(),
        Column::build("Directory_").string(72).build(),
        Column::build("Attributes").int16().build(),
        Column::build("Condition").string(255).nullable().build(),
        Column::build("KeyPath").string(72).nullable().build(),
    ]).unwrap();
    builder.insert_rows("Component", vec![
        vec![Value::from("Comp1"), Value::Null, Value::from("INSTALLDIR"),
             Value::Int(0), Value::Null, Value::from("hello.txt")],
    ]).unwrap();

    builder.create_table("File", vec![
        Column::build("File").string(72).primary_key().build(),
        Column::build("Component_").string(72).build(),
        Column::build("FileName").string(255).build(),
        Column::build("FileSize").int32().build(),
        Column::build("Sequence").int16().build(),
    ]).unwrap();
    let content = b"Hello World\r\n";
    builder.insert_rows("File", vec![
        vec![Value::from("hello.txt"), Value::from("Comp1"), Value::from("hello.txt"),
             Value::Int(content.len() as i32), Value::Int(1)],
    ]).unwrap();

    builder.create_table("Feature", vec![
        Column::build("Feature").string(38).primary_key().build(),
        Column::build("Feature_Parent").string(38).nullable().build(),
        Column::build("Title").string(64).nullable().build(),
        Column::build("Description").string(255).nullable().build(),
        Column::build("Display").int16().nullable().build(),
        Column::build("Level").int16().build(),
        Column::build("Directory_").string(72).nullable().build(),
        Column::build("Attributes").int16().nullable().build(),
    ]).unwrap();
    builder.insert_rows("Feature", vec![
        vec![Value::from("Feat1"), Value::Null, Value::from("Main"),
             Value::from("Main feature"), Value::Int(1), Value::Int(1),
             Value::from("INSTALLDIR"), Value::Null],
    ]).unwrap();

    builder.create_table("FeatureComponents", vec![
        Column::build("Feature_").string(38).primary_key().build(),
        Column::build("Component_").string(72).primary_key().build(),
    ]).unwrap();
    builder.insert_rows("FeatureComponents", vec![
        vec![Value::from("Feat1"), Value::from("Comp1")],
    ]).unwrap();

    builder.create_table("Media", vec![
        Column::build("DiskId").int16().primary_key().build(),
        Column::build("LastSequence").int16().build(),
        Column::build("DiskPrompt").string(64).nullable().build(),
        Column::build("Cabinet").string(255).nullable().build(),
        Column::build("VolumeLabel").string(32).nullable().build(),
        Column::build("Source").string(72).nullable().build(),
    ]).unwrap();
    builder.insert_rows("Media", vec![
        vec![Value::Int(1), Value::Int(1), Value::Null,
             Value::from("#vel.cab"), Value::Null, Value::Null],
    ]).unwrap();

    // Build MSI WITHOUT the cabinet stream (we'll add it via cfb)
    let msi_data = builder.build().unwrap();
    println!("Base MSI (no cab stream): {} bytes", msi_data.len());

    // Step 2: Open with cfb and add the cabinet stream
    let cab = build_cabinet(&[CabinetFile {
        name: "hello.txt".to_string(),
        data: content.to_vec(),
    }]);
    println!("Cabinet: {} bytes", cab.len());

    let mut cursor = Cursor::new(msi_data);
    {
        let mut comp = cfb::CompoundFile::open(&mut cursor).unwrap();
        // Add cabinet stream with RAW name
        comp.create_stream("vel.cab").unwrap().write_all(&cab).unwrap();
    }
    
    // Get the result
    let result = cursor.into_inner();
    std::fs::write("cfb_test.msi", &result).unwrap();
    println!("CFB MSI: {} bytes", result.len());

    // List streams
    println!("\n=== Streams ===");
    let mut comp2 = cfb::CompoundFile::open(Cursor::new(&result)).unwrap();
    let paths: Vec<_> = comp2.walk()
        .map(|e| (e.path().to_string_lossy().to_string(), e.is_stream()))
        .collect();
    for (p, s) in &paths {
        println!("  {} {}", if *s { "S" } else { "D" }, p);
    }

    // Test with msiexec
    println!("\n=== msiexec test ===");
    let output = std::process::Command::new("msiexec")
        .args(&["/i", "cfb_test.msi", "/qn", "/l*v", "cfb_test_log.txt"])
        .output().unwrap();
    println!("Exit: {}", output.status.code().unwrap_or(-1));

    // Check log
    if let Ok(log) = std::fs::read_to_string("cfb_test_log.txt") {
        for line in log.lines() {
            if line.contains("returning") || line.contains("Note: 1:") 
                || (line.contains("Error") && !line.contains("Error 0"))
                || line.contains("cabinet") || line.contains("Cabinet")
                || line.contains("hello") {
                println!("  {}", line.trim());
            }
        }
    }

    // Check if file was installed
    // (We don't know the exact install path, but check common locations)
    
    // Cleanup
    let _ = std::fs::remove_file("cfb_test.msi");
    let _ = std::fs::remove_file("cfb_test_log.txt");
}
