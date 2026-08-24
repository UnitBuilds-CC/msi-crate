/// Debug: check cabinet stream in MSI
use velocity_msi::{MsiBuilder, Column, Value, CabinetFile, build_cabinet};
use std::io::Read;

fn main() {
    let mut builder = MsiBuilder::new();
    builder.set_title("Velocity Test");
    builder.set_author("Velocity");
    builder.set_template("x64", 1033);

    builder.create_table("Property", vec![
        Column::build("Property").string(72).primary_key().build(),
        Column::build("Value").string(255).nullable().build(),
    ]).unwrap();

    let product_code = "{12345678-1234-1234-1234-123456789ABC}";
    let upgrade_code = "{87654321-4321-4321-4321-CBA987654321}";

    builder.insert_rows("Property", vec![
        vec![Value::from("ProductName"), Value::from("Velocity Test")],
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
        vec![Value::from("INSTALLDIR"), Value::from("TARGETDIR"), Value::from("VelTest:.")],
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

    let cab = build_cabinet(&[CabinetFile {
        name: "hello.txt".to_string(),
        data: content.to_vec(),
    }]);
    
    println!("Cabinet data: {} bytes", cab.len());
    println!("Cabinet header: {:02x?}", &cab[..36.min(cab.len())]);
    
    builder.insert_rows("Media", vec![
        vec![Value::Int(1), Value::Int(1), Value::Null,
             Value::from("#vel.cab"), Value::Null, Value::Null],
    ]).unwrap();
    builder.add_stream("vel.cab".to_string(), cab.clone());

    let msi = builder.build().unwrap();
    std::fs::write("debug_install.msi", &msi).unwrap();
    println!("MSI: {} bytes", msi.len());

    // Save standalone cabinet for testing
    std::fs::write("debug_vel.cab", &cab).unwrap();

    // Open with cfb and list streams
    println!("\n=== OLE streams ===");
    let mut comp = cfb::CompoundFile::open(std::io::Cursor::new(&msi)).unwrap();
    
    let stream_paths: Vec<_> = comp.walk()
        .map(|e| (e.path().to_string_lossy().to_string(), e.is_stream()))
        .collect();
    
    for (path, is_stream) in &stream_paths {
        println!("  {} {}", if *is_stream { "S" } else { "D" }, path);
    }
    
    // Read cabinet stream data
    for (path, is_stream) in &stream_paths {
        if *is_stream && (path.contains("vel") || path.contains("cab")) {
            let mut stream = comp.open_stream(path.as_str()).unwrap();
            let mut data = Vec::new();
            stream.read_to_end(&mut data).unwrap();
            println!("\n  Stream '{}' = {} bytes", path, data.len());
            println!("  Header: {:02x?}", &data[..36.min(data.len())]);
            if data.len() > 36 {
                println!("  After CFHEADER: {:02x?}", &data[36..(36+20).min(data.len())]);
            }
        }
    }

    // Test standalone cabinet with expand
    println!("\n=== Testing standalone cabinet ===");
    let _ = std::fs::remove_dir_all("debug_cab_output");
    let output = std::process::Command::new("expand")
        .args(&["debug_vel.cab", "-F:*", "debug_cab_output"])
        .output();
    match output {
        Ok(o) => {
            println!("expand exit: {}", o.status.code().unwrap_or(-1));
            println!("stdout: {}", String::from_utf8_lossy(&o.stdout));
            println!("stderr: {}", String::from_utf8_lossy(&o.stderr));
        }
        Err(e) => println!("expand error: {}", e),
    }
}
