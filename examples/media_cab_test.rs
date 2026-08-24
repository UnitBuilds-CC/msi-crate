/// Test: Media table + cabinet stream, trying ALL name combinations
/// cargo run --example media_cab_test -p velocity-msi
use std::io::{Cursor, Write};
use velocity_msi::{Column, MsiBuilder, Value, encode_stream_name};

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

fn test_combo(label: &str, media_cab_value: &str, stream_name: &str) -> i32 {
    let pc = make_uuid();
    let uc = make_uuid();
    let cab_data = std::fs::read("C:\\temp\\good.cab").unwrap();
    let file_content = std::fs::read("C:\\temp\\testfile.txt").unwrap();

    let mut builder = MsiBuilder::new();
    builder.set_title("Media Cab Test");
    builder.set_author("Velocity");
    builder.set_template("Intel", 1033);

    // Property
    builder.create_table("Property", vec![
        Column::build("Property").string(72).primary_key().build(),
        Column::build("Value").nullable().string(255).localizable().build(),
    ]).unwrap();
    builder.insert_rows("Property", vec![
        vec![Value::from("ProductName"), Value::from("Media Cab Test")],
        vec![Value::from("ProductVersion"), Value::from("1.0.0")],
        vec![Value::from("Manufacturer"), Value::from("Velocity")],
        vec![Value::from("ProductCode"), Value::from(pc.as_str())],
        vec![Value::from("UpgradeCode"), Value::from(uc.as_str())],
        vec![Value::from("ProductLanguage"), Value::from("1033")],
    ]).unwrap();

    // Directory
    builder.create_table("Directory", vec![
        Column::build("Directory").string(72).primary_key().build(),
        Column::build("Directory_Parent").nullable().string(72).build(),
        Column::build("DefaultDir").nullable().string(255).localizable().build(),
    ]).unwrap();
    builder.insert_rows("Directory", vec![
        vec![Value::from("TARGETDIR"), Value::Null, Value::from("SourceDir")],
        vec![Value::from("ProgramFilesFolder"), Value::from("TARGETDIR"), Value::from("PFiles")],
        vec![Value::from("INSTALLDIR"), Value::from("ProgramFilesFolder"), Value::from("VelMediaTest")],
    ]).unwrap();

    // Component
    builder.create_table("Component", vec![
        Column::build("Component").string(72).primary_key().build(),
        Column::build("ComponentId").nullable().string(38).build(),
        Column::build("Directory_").string(72).build(),
        Column::build("Attributes").int16().build(),
        Column::build("Condition").nullable().string(255).build(),
        Column::build("KeyPath").nullable().string(72).build(),
    ]).unwrap();
    builder.insert_rows("Component", vec![
        vec![Value::from("MainComp"), Value::Null, Value::from("INSTALLDIR"),
             Value::Int(0), Value::Null, Value::from("MainFile")],
    ]).unwrap();

    // Feature
    builder.create_table("Feature", vec![
        Column::build("Feature").string(38).primary_key().build(),
        Column::build("Feature_Parent").nullable().string(38).build(),
        Column::build("Title").nullable().string(64).localizable().build(),
        Column::build("Description").nullable().string(255).localizable().build(),
        Column::build("Display").nullable().int16().build(),
        Column::build("Level").int16().build(),
        Column::build("Directory_").nullable().string(72).build(),
        Column::build("Attributes").int16().build(),
    ]).unwrap();
    builder.insert_rows("Feature", vec![
        vec![Value::from("MainFeat"), Value::Null, Value::from("Complete"),
             Value::Null, Value::Null, Value::Int(1), Value::Null, Value::Int(0)],
    ]).unwrap();

    // FeatureComponents
    builder.create_table("FeatureComponents", vec![
        Column::build("Feature_").string(38).primary_key().build(),
        Column::build("Component_").string(72).primary_key().build(),
    ]).unwrap();
    builder.insert_rows("FeatureComponents", vec![
        vec![Value::from("MainFeat"), Value::from("MainComp")],
    ]).unwrap();

    // File
    builder.create_table("File", vec![
        Column::build("File_").string(72).primary_key().build(),
        Column::build("Component_").string(72).build(),
        Column::build("FileName").string(255).localizable().build(),
        Column::build("FileSize").nullable().int32().build(),
        Column::build("Sequence").int16().build(),
    ]).unwrap();
    builder.insert_rows("File", vec![
        vec![Value::from("MainFile"), Value::from("MainComp"),
             Value::from("testfile.txt"), Value::Int(file_content.len() as i32), Value::Int(1)],
    ]).unwrap();

    // InstallExecuteSequence
    builder.create_table("InstallExecuteSequence", vec![
        Column::build("Action").string(72).primary_key().build(),
        Column::build("Condition").nullable().string(255).build(),
        Column::build("Sequence").nullable().int16().build(),
    ]).unwrap();
    builder.insert_rows("InstallExecuteSequence", vec![
        vec![Value::from("CostInitialize"), Value::Null, Value::Int(800)],
        vec![Value::from("FileCost"), Value::Null, Value::Int(900)],
        vec![Value::from("CostFinalize"), Value::Null, Value::Int(1000)],
        vec![Value::from("InstallValidate"), Value::Null, Value::Int(1400)],
        vec![Value::from("InstallInitialize"), Value::Null, Value::Int(1500)],
        vec![Value::from("InstallFinalize"), Value::Null, Value::Int(6600)],
    ]).unwrap();

    // InstallUISequence
    builder.create_table("InstallUISequence", vec![
        Column::build("Action").string(72).primary_key().build(),
        Column::build("Condition").nullable().string(255).build(),
        Column::build("Sequence").nullable().int16().build(),
    ]).unwrap();
    builder.insert_rows("InstallUISequence", vec![
        vec![Value::from("CostInitialize"), Value::Null, Value::Int(800)],
        vec![Value::from("CostFinalize"), Value::Null, Value::Int(1000)],
        vec![Value::from("ExecuteAction"), Value::Null, Value::Int(1300)],
    ]).unwrap();

    // Media table with the given cabinet value
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
             Value::from(media_cab_value), Value::Null, Value::Null],
    ]).unwrap();

    let msi_data = builder.build().unwrap();

    // Add cabinet stream with the given name
    let mut buf = msi_data.clone();
    {
        let cursor = Cursor::new(&mut buf);
        let mut comp = cfb::CompoundFile::open(cursor).unwrap();
        {
            let mut s = comp.create_stream(stream_name).unwrap();
            s.write_all(&cab_data).unwrap();
        }
        comp.flush().unwrap();
    }

    let safe_label = label.replace(' ', "_").replace('#', "H").replace('.', "d");
    let path = format!("C:\\temp\\media_{}.msi", safe_label);
    let log_path = format!("C:\\temp\\media_{}.log", safe_label);
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&log_path);
    std::fs::write(&path, &buf).unwrap();

    let output = std::process::Command::new("msiexec")
        .args(&["/i", &path, "/qn", "/l*v", &log_path])
        .output().unwrap();
    let ec = output.status.code().unwrap_or(-1);

    // Check log for key errors
    let mut error_info = String::new();
    if let Ok(log) = std::fs::read_to_string(&log_path) {
        for line in log.lines() {
            if line.contains("2725") { error_info = "2725:cabinet-not-found".to_string(); break; }
            if line.contains("2203") { error_info = "2203:database-error".to_string(); break; }
        }
    }

    println!("  {:40} | Media={:20} | Stream={:30} | EC={} {}",
        label, media_cab_value, stream_name, ec, error_info);
    
    // Uninstall if success
    if ec == 0 {
        let install_dir = "C:\\Program Files (x86)\\VelMediaTest";
        if std::path::Path::new(install_dir).exists() {
            if let Ok(entries) = std::fs::read_dir(install_dir) {
                for entry in entries.flatten() {
                    println!("    INSTALLED: {}", entry.path().display());
                }
            }
        }
        let _ = std::process::Command::new("msiexec")
            .args(&["/x", &pc, "/qn"]).output();
    }
    
    ec
}

fn main() {
    println!("=== MEDIA TABLE + CABINET STREAM NAME COMBINATIONS ===\n");

    let enc_hash = encode_stream_name("#velcab.cab", false);
    let enc_nohash = encode_stream_name("velcab.cab", false);

    let combos: Vec<(&str, &str, &str)> = vec![
        // (label, media_value, stream_name)
        ("hash_media_hash_stream",    "#velcab.cab", "#velcab.cab"),
        ("hash_media_enc_hash_stream", "#velcab.cab", enc_hash.as_str()),
        ("hash_media_raw_stream",     "#velcab.cab", "velcab.cab"),
        ("hash_media_enc_nohash",     "#velcab.cab", enc_nohash.as_str()),
        ("nohash_media_hash_stream",  "velcab.cab",  "#velcab.cab"),
        ("nohash_media_enc_stream",   "velcab.cab",  enc_hash.as_str()),
        ("nohash_media_raw_stream",   "velcab.cab",  "velcab.cab"),
        ("nohash_media_enc_nohash",   "velcab.cab",  enc_nohash.as_str()),
    ];

    println!("{:-<120}", "");
    println!("{:40} | {:20} | {:30} | Result", "Test", "Media Cabinet", "Stream Name");
    println!("{:-<120}", "");

    for (label, media_val, stream_name) in &combos {
        test_combo(label, media_val, stream_name);
    }

    println!("{:-<120}", "");
    println!("\nDone!");
}
