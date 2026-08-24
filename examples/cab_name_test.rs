/// Test: try installing with cabinet stream under different names
/// to find which one msiexec expects.
use std::io::{Write, Cursor};
use velocity_msi::{MsiBuilder, Column, Value, CabinetFile, build_cabinet, encode_stream_name};

fn build_test_msi(cab_stream_name: &str) -> Vec<u8> {
    let mut builder = MsiBuilder::new();
    builder.set_title("Cab Name Test");
    builder.set_author("Velocity");
    builder.set_template("x64", 1033);

    builder.create_table("Property", vec![
        Column::build("Property").string(72).primary_key().build(),
        Column::build("Value").string(255).nullable().build(),
    ]).unwrap();

    let product_code = format!("{{{:08X}-1234-1234-1234-123456789ABC}}", 
        cab_stream_name.len() as u32 * 0x12345678);
    let upgrade_code = "{87654321-4321-4321-4321-CBA987654321}";

    builder.insert_rows("Property", vec![
        vec![Value::from("ProductName"), Value::from("Cab Test")],
        vec![Value::from("ProductVersion"), Value::from("1.0.0")],
        vec![Value::from("Manufacturer"), Value::from("Velocity")],
        vec![Value::from("ProductCode"), Value::from(product_code.as_str())],
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
        vec![Value::from("INSTALLDIR"), Value::from("TARGETDIR"), Value::from("TestDir:.")],
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

    let cab = build_cabinet(&[CabinetFile {
        name: "hello.txt".to_string(),
        data: content.to_vec(),
    }]);
    
    // Add cabinet stream with the specified name (bypassing build()'s encoding)
    builder.add_stream(cab_stream_name.to_string(), cab);

    builder.build().unwrap()
}

fn main() {
    let content = b"Hello World\r\n";
    
    // Test 1: Raw name "vel.cab" (build() will encode it)
    println!("=== Test 1: build() encodes 'vel.cab' ===");
    let encoded = encode_stream_name("vel.cab", false);
    println!("  Encoded name: {:?}", encoded);
    let msi = build_test_msi("vel.cab");
    test_msi(&msi, "test1.msi");

    // Test 2: Use cfb to add raw "vel.cab" stream to a base MSI
    println!("\n=== Test 2: cfb adds raw 'vel.cab' ===");
    {
        // Build base MSI without cabinet stream
        let mut builder = MsiBuilder::new();
        builder.set_title("CFB Raw Test");
        builder.set_author("Velocity");
        builder.set_template("x64", 1033);
    
        builder.create_table("Property", vec![
            Column::build("Property").string(72).primary_key().build(),
            Column::build("Value").string(255).nullable().build(),
        ]).unwrap();
    
        let product_code = "{DDDDDDDD-EEEE-FFFF-0000-111111111111}";
        let upgrade_code = "{87654321-4321-4321-4321-CBA987654321}";
    
        builder.insert_rows("Property", vec![
            vec![Value::from("ProductName"), Value::from("CFB Raw Test")],
            vec![Value::from("ProductVersion"), Value::from("1.0.0")],
            vec![Value::from("Manufacturer"), Value::from("Velocity")],
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
            vec![Value::from("INSTALLDIR"), Value::from("TARGETDIR"), Value::from("TestDir:.")],
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
    
        // Build WITHOUT cabinet stream
        let base_msi = builder.build().unwrap();
        
        // Add cabinet via cfb with raw name
        let cab = build_cabinet(&[CabinetFile {
            name: "hello.txt".to_string(),
            data: content.to_vec(),
        }]);
        
        let mut cursor = Cursor::new(base_msi);
        {
            let mut comp = cfb::CompoundFile::open(&mut cursor).unwrap();
            comp.create_stream("vel.cab").unwrap().write_all(&cab).unwrap();
        }
        let result = cursor.into_inner();
        std::fs::write("test2.msi", &result).unwrap();
        println!("  MSI: {} bytes", result.len());
    }
    test_msi_file("test2.msi");

    // Cleanup
    for f in &["test1.msi", "test2.msi", "test1_log.txt", "test2_log.txt"] {
        let _ = std::fs::remove_file(f);
    }
}

fn test_msi(msi: &[u8], name: &str) {
    std::fs::write(name, msi).unwrap();
    println!("  MSI: {} bytes", msi.len());
    
    // List streams
    let comp = cfb::CompoundFile::open(Cursor::new(msi)).unwrap();
    for entry in comp.walk() {
        if entry.is_stream() {
            let p = entry.path().to_string_lossy();
            if p.contains("vel") || p.contains("cab") {
                println!("  Cabinet stream: {:?}", p);
            }
        }
    }
    
    let log_name = name.replace(".msi", "_log.txt");
    let output = std::process::Command::new("msiexec")
        .args(&["/i", name, "/qn", "/l*v", &log_name])
        .output().unwrap();
    println!("  Exit: {}", output.status.code().unwrap_or(-1));
    
    if let Ok(log) = std::fs::read_to_string(&log_name) {
        for line in log.lines() {
            if line.contains("2725") || line.contains("1708") || line.contains("returning") {
                println!("  Log: {}", line.trim());
            }
        }
    }
    let _ = std::fs::remove_file(name);
    let _ = std::fs::remove_file(&log_name);
}

fn test_msi_file(name: &str) {
    let log_name = name.replace(".msi", "_log.txt");
    let output = std::process::Command::new("msiexec")
        .args(&["/i", name, "/qn", "/l*v", &log_name])
        .output().unwrap();
    println!("  Exit: {}", output.status.code().unwrap_or(-1));
    
    if let Ok(log) = std::fs::read_to_string(&log_name) {
        for line in log.lines() {
            if line.contains("2725") || line.contains("1708") || line.contains("returning") {
                println!("  Log: {}", line.trim());
            }
        }
    }
}
