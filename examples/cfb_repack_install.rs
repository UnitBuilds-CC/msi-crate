/// Take the failing MSI (with InstallExecuteSequence) and repackage with cfb crate.
/// This isolates whether the bug is in our OLE writer or the table data.
use velocity_msi::{MsiBuilder, Column, Value, CabinetFile, build_cabinet};
use std::io::{Read, Write, Cursor};

fn main() {
    let content = b"Hello from Velocity MSI!\r\n";

    let mut builder = MsiBuilder::new();
    builder.set_title("CFB Repack Test");
    builder.set_author("Velocity");
    builder.set_template("Intel", 1033);

    // Property
    builder.create_table("Property", vec![
        Column::build("Property").string(72).primary_key().build(),
        Column::build("Value").string(1024).build(),
    ]).unwrap();
    builder.insert_rows("Property", vec![
        vec![Value::from("ProductName"), Value::from("CFB Repack Test")],
        vec![Value::from("ProductVersion"), Value::from("1.0.0")],
        vec![Value::from("Manufacturer"), Value::from("Velocity Corp")],
        vec![Value::from("ProductCode"), Value::from("{AADDD4E4-3A12-4B7D-A4E0-5A2F8B3C6D90}")],
        vec![Value::from("UpgradeCode"), Value::from("{BBDDE5F5-4B23-5C8E-B5F1-6B3A9C4D7EA1}")],
        vec![Value::from("ProductLanguage"), Value::from("1033")],
    ]).unwrap();

    // Directory
    builder.create_table("Directory", vec![
        Column::build("Directory").string(72).primary_key().build(),
        Column::build("Directory_Parent").string(72).nullable().build(),
        Column::build("DefaultDir").string(255).primary_key().build(),
    ]).unwrap();
    builder.insert_rows("Directory", vec![
        vec![Value::from("TARGETDIR"), Value::Null, Value::from("SourceDir")],
        vec![Value::from("INSTALLDIR"), Value::from("TARGETDIR"), Value::from("VelTest:VelTest")],
    ]).unwrap();

    // Component
    builder.create_table("Component", vec![
        Column::build("Component").string(72).primary_key().build(),
        Column::build("ComponentId").string(38).nullable().build(),
        Column::build("Directory_").string(72).build(),
        Column::build("Attributes").int16().build(),
        Column::build("Condition").string(255).nullable().build(),
        Column::build("KeyPath").string(72).nullable().build(),
    ]).unwrap();
    builder.insert_rows("Component", vec![
        vec![Value::from("MainComp"), Value::Null, Value::from("INSTALLDIR"),
             Value::Int(0), Value::Null, Value::from("hello.txt")],
    ]).unwrap();

    // File
    builder.create_table("File", vec![
        Column::build("File").string(72).primary_key().build(),
        Column::build("Component_").string(72).build(),
        Column::build("FileName").string(255).build(),
        Column::build("FileSize").int32().build(),
        Column::build("Version").string(72).nullable().build(),
        Column::build("Language").string(20).nullable().build(),
        Column::build("Attributes").int16().nullable().build(),
        Column::build("Sequence").int16().build(),
    ]).unwrap();
    builder.insert_rows("File", vec![
        vec![Value::from("hello.txt"), Value::from("MainComp"), Value::from("hello.txt"),
             Value::Int(content.len() as i32),
             Value::Null, Value::Null, Value::Null, Value::Int(1)],
    ]).unwrap();

    // Feature
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
        vec![Value::from("Complete"), Value::Null, Value::from("Complete"),
             Value::from("All files"), Value::Int(1), Value::Int(1),
             Value::Null, Value::Null],
    ]).unwrap();

    // FeatureComponents
    builder.create_table("FeatureComponents", vec![
        Column::build("Feature_").string(38).primary_key().build(),
        Column::build("Component_").string(72).primary_key().build(),
    ]).unwrap();
    builder.insert_rows("FeatureComponents", vec![
        vec![Value::from("Complete"), Value::from("MainComp")],
    ]).unwrap();

    // Media + cabinet
    let cab = build_cabinet(&[CabinetFile {
        name: "hello.txt".to_string(),
        data: content.to_vec(),
    }]);
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
    builder.add_stream("vel.cab".to_string(), cab);

    // InstallExecuteSequence
    builder.create_table("InstallExecuteSequence", vec![
        Column::build("Action").string(72).primary_key().build(),
        Column::build("Condition").string(255).nullable().build(),
        Column::build("Sequence").int16().nullable().build(),
    ]).unwrap();
    builder.insert_rows("InstallExecuteSequence", vec![
        vec![Value::from("CostFinalize"), Value::Null, Value::Int(1000)],
        vec![Value::from("CostInitialize"), Value::Null, Value::Int(800)],
        vec![Value::from("FileCost"), Value::Null, Value::Int(900)],
        vec![Value::from("InstallFiles"), Value::Null, Value::Int(4000)],
        vec![Value::from("InstallFinalize"), Value::Null, Value::Int(6600)],
        vec![Value::from("InstallInitialize"), Value::Null, Value::Int(1500)],
        vec![Value::from("InstallValidate"), Value::Null, Value::Int(1400)],
    ]).unwrap();

    // Build with our custom OLE writer
    let custom_msi = builder.build().unwrap();
    std::fs::write("custom_repack.msi", &custom_msi).unwrap();
    println!("Custom OLE MSI: {} bytes", custom_msi.len());

    // Now repackage with cfb crate - write to file directly
    let cfb_file = std::fs::File::create("cfb_repack.msi").unwrap();
    let mut cfb_out = cfb::CompoundFile::create(cfb_file).unwrap();

    // Read the custom MSI with cfb and copy all streams
    let mut cfb_in = cfb::CompoundFile::open(Cursor::new(&custom_msi)).unwrap();
    let entries: Vec<(String, bool)> = cfb_in.walk()
        .map(|e| (e.path().to_string_lossy().to_string(), e.is_stream()))
        .collect();

    for (path, is_stream) in &entries {
        if !is_stream { continue; }
        let mut data = Vec::new();
        cfb_in.open_stream(path.as_str()).unwrap().read_to_end(&mut data).unwrap();
        println!("  Stream: {} ({} bytes)", path, data.len());
        cfb_out.create_stream(path.as_str()).unwrap().write_all(&data).unwrap();
    }
    drop(cfb_out); // flush and close
    let cfb_size = std::fs::metadata("cfb_repack.msi").unwrap().len();
    println!("\nCFB repackaged MSI: {} bytes", cfb_size);

    // Test both
    for (label, fname) in &[("Custom OLE", "custom_repack.msi"), ("CFB repack", "cfb_repack.msi")] {
        let _ = std::fs::remove_dir_all("C:\\VelTest");
        let log = fname.replace(".msi", ".log");
        let output = std::process::Command::new("msiexec")
            .args(&["/i", fname, "/qn", "/l*v", &log])
            .output().unwrap();
        let exit = output.status.code().unwrap_or(-1);
        let installed = std::path::Path::new("C:\\VelTest").exists();
        println!("\n{:20} exit={} installed={}", label, exit, installed);

        if exit != 0 {
            if let Ok(logtext) = std::fs::read_to_string(&log) {
                for line in logtext.lines() {
                    if (line.contains("Error ") && !line.contains("Error 0") && !line.contains("2205") && !line.contains("2228"))
                        || line.contains("return value 3")
                    {
                        println!("  {}", line.trim());
                    }
                }
            }
        }
    }
}
