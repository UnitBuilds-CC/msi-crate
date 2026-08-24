/// Binary comparison: msi crate reference vs velocity-msi
use std::io::{Cursor, Read, Write};

fn main() {
    // Create reference MSI using msi crate
    let ref_msi = create_ref_msi();
    std::fs::write("ref_msi.msi", &ref_msi).unwrap();
    println!("Reference MSI: {} bytes", ref_msi.len());

    // Create our MSI with same tables
    let our_msi = create_our_msi();
    std::fs::write("our_msi.msi", &our_msi).unwrap();
    println!("Our MSI: {} bytes", our_msi.len());

    // Extract and compare all streams
    let ref_streams = extract_streams(&ref_msi, "REF");
    let our_streams = extract_streams(&our_msi, "OUR");

    println!("\n=== Stream comparison ===");
    for (name, ref_data) in &ref_streams {
        if let Some(our_data) = our_streams.get(name) {
            if ref_data == our_data {
                println!("MATCH: {} ({} bytes)", name, ref_data.len());
            } else {
                println!("DIFF: {} (ref={} bytes, our={} bytes)", name, ref_data.len(), our_data.len());
                let mut diffs = 0;
                for i in 0..ref_data.len().min(our_data.len()) {
                    if ref_data[i] != our_data[i] {
                        if diffs < 5 {
                            println!("  diff at byte {}: ref=0x{:02X}, our=0x{:02X}", i, ref_data[i], our_data[i]);
                            if diffs == 0 {
                                let start = i.saturating_sub(4);
                                let end = (i + 32).min(ref_data.len()).min(our_data.len());
                                print!("  REF: ");
                                for b in &ref_data[start..end] { print!("{:02X} ", b); }
                                println!();
                                print!("  OUR: ");
                                for b in &our_data[start..end] { print!("{:02X} ", b); }
                                println!();
                            }
                        }
                        diffs += 1;
                    }
                }
                println!("  ...total {} byte diffs{}", diffs,
                    if ref_data.len() != our_data.len() { " (LENGTH DIFFERS!)" } else { "" });
            }
        } else {
            println!("ONLY IN REF: {} ({} bytes)", name, ref_data.len());
        }
    }
    for name in our_streams.keys() {
        if !ref_streams.contains_key(name) {
            println!("ONLY IN OURS: {}", name);
        }
    }

    // Test both with msiexec
    for code in &["{EEEEEEEE-EEEE-EEEE-EEEE-EEEEEEEEEEEE}", "{FFFFFFFF-FFFF-FFFF-FFFF-FFFFFFFFFFFF}"] {
        let _ = std::process::Command::new("msiexec")
            .args(&["/x", code, "/qn", "/norestart"]).output();
    }
    std::thread::sleep(std::time::Duration::from_secs(1));

    for (name, _code) in &[("ref", "{EEEEEEEE-EEEE-EEEE-EEEE-EEEEEEEEEEEE}"),
                            ("our", "{FFFFFFFF-FFFF-FFFF-FFFF-FFFFFFFFFFFF}")] {
        let logfile = format!("{}_log.txt", name);
        let msi_file = format!("{}_msi.msi", name);
        // Copy files to expected names
        let src = if *name == "ref" { "ref_msi.msi" } else { "our_msi.msi" };
        std::fs::copy(src, &msi_file).unwrap();

        println!("\n=== Testing {} MSI ===", name.to_uppercase());
        let output = std::process::Command::new("msiexec")
            .args(&["/i", &msi_file, "/qn", "/l*v", &logfile]).output().unwrap();
        println!("{} exit: {}", name.to_uppercase(), output.status.code().unwrap_or(-1));

        if let Ok(log) = std::fs::read_to_string(&logfile) {
            for line in log.lines() {
                if line.contains("2705") || line.contains("Could not be linked")
                    || line.contains("DEBUG: Error")
                    || (line.contains("Return value") && line.contains("INSTALL"))
                {
                    println!("  {}", line.trim());
                }
            }
        }
    }
}

fn create_ref_msi() -> Vec<u8> {
    use msi::{Column, Insert, Package, PackageType, CodePage, Value};

    let buf: Vec<u8> = Vec::new();
    let cursor = Cursor::new(buf);
    let mut pkg = Package::create(PackageType::Installer, cursor).unwrap();
    pkg.set_database_codepage(CodePage::Windows1252);

    // Set summary info
    pkg.summary_info_mut().set_title("Velocity Ref Test");
    pkg.summary_info_mut().set_author("Velocity");

    // Property table
    pkg.create_table("Property", vec![
        Column::build("Property").primary_key().string(72),
        Column::build("Value").nullable().string(255),
    ]).unwrap();
    pkg.insert_rows(Insert::into("Property")
        .row(vec![Value::Str("ProductName".into()), Value::Str("Velocity Ref Test".into())])
        .row(vec![Value::Str("ProductVersion".into()), Value::Str("1.0.0".into())])
        .row(vec![Value::Str("Manufacturer".into()), Value::Str("Velocity Corp".into())])
        .row(vec![Value::Str("ProductCode".into()), Value::Str("{EEEEEEEE-EEEE-EEEE-EEEE-EEEEEEEEEEEE}".into())])
        .row(vec![Value::Str("UpgradeCode".into()), Value::Str("{87654321-4321-4321-4321-CBA987654321}".into())])
        .row(vec![Value::Str("ProductLanguage".into()), Value::Str("1033".into())])
    ).unwrap();

    // Directory table
    pkg.create_table("Directory", vec![
        Column::build("Directory").primary_key().string(72),
        Column::build("Directory_Parent").nullable().string(72),
        Column::build("DefaultDir").primary_key().string(255),
    ]).unwrap();
    pkg.insert_rows(Insert::into("Directory")
        .row(vec![Value::Str("TARGETDIR".into()), Value::Null, Value::Str("SourceDir".into())])
        .row(vec![Value::Str("INSTALLDIR".into()), Value::Str("TARGETDIR".into()), Value::Str("VelTest:VelTest".into())])
    ).unwrap();

    // Component
    pkg.create_table("Component", vec![
        Column::build("Component").primary_key().string(72),
        Column::build("ComponentId").nullable().string(38),
        Column::build("Directory_").string(72),
        Column::build("Attributes").int16(),
        Column::build("Condition").nullable().string(255),
        Column::build("KeyPath").nullable().string(72),
    ]).unwrap();
    pkg.insert_rows(Insert::into("Component")
        .row(vec![
            Value::Str("Comp1".into()), Value::Null, Value::Str("INSTALLDIR".into()),
            Value::Int(0), Value::Null, Value::Str("hello.txt".into()),
        ])
    ).unwrap();

    // File (8 columns)
    let content = b"Hello World from Velocity Installer!\r\n";
    pkg.create_table("File", vec![
        Column::build("File").primary_key().string(72),
        Column::build("Component_").string(72),
        Column::build("FileName").string(255),
        Column::build("FileSize").int32(),
        Column::build("Sequence").int16(),
        Column::build("Version").nullable().string(72),
        Column::build("Language").nullable().string(20),
        Column::build("Attributes").nullable().int16(),
    ]).unwrap();
    pkg.insert_rows(Insert::into("File")
        .row(vec![
            Value::Str("hello.txt".into()), Value::Str("Comp1".into()),
            Value::Str("hello.txt".into()), Value::Int(content.len() as i32),
            Value::Int(1), Value::Null, Value::Null, Value::Null,
        ])
    ).unwrap();

    // Feature
    pkg.create_table("Feature", vec![
        Column::build("Feature").primary_key().string(38),
        Column::build("Feature_Parent").nullable().string(38),
        Column::build("Title").nullable().string(64),
        Column::build("Description").nullable().string(255),
        Column::build("Display").nullable().int16(),
        Column::build("Level").int16(),
        Column::build("Directory_").nullable().string(72),
        Column::build("Attributes").nullable().int16(),
    ]).unwrap();
    pkg.insert_rows(Insert::into("Feature")
        .row(vec![
            Value::Str("Feat1".into()), Value::Null, Value::Str("Main".into()),
            Value::Str("Main feature".into()), Value::Int(1), Value::Int(1),
            Value::Null, Value::Null,
        ])
    ).unwrap();

    // FeatureComponents
    pkg.create_table("FeatureComponents", vec![
        Column::build("Feature_").primary_key().string(38),
        Column::build("Component_").primary_key().string(72),
    ]).unwrap();
    pkg.insert_rows(Insert::into("FeatureComponents")
        .row(vec![Value::Str("Feat1".into()), Value::Str("Comp1".into())])
    ).unwrap();

    // Media
    pkg.create_table("Media", vec![
        Column::build("DiskId").primary_key().int16(),
        Column::build("LastSequence").int16(),
        Column::build("DiskPrompt").nullable().string(64),
        Column::build("Cabinet").nullable().string(255),
        Column::build("VolumeLabel").nullable().string(32),
        Column::build("Source").nullable().string(72),
    ]).unwrap();
    pkg.insert_rows(Insert::into("Media")
        .row(vec![
            Value::Int(1), Value::Int(1), Value::Null,
            Value::Str("#vel.cab".into()), Value::Null, Value::Null,
        ])
    ).unwrap();

    // InstallExecuteSequence
    pkg.create_table("InstallExecuteSequence", vec![
        Column::build("Action").primary_key().string(72),
        Column::build("Condition").nullable().string(255),
        Column::build("Sequence").nullable().int16(),
    ]).unwrap();
    let actions: Vec<(&str, i32)> = vec![
        ("LaunchConditions", 100), ("CostInitialize", 800), ("FileCost", 900),
        ("CostFinalize", 1000), ("InstallValidate", 1400), ("InstallInitialize", 1500),
        ("ProcessComponents", 1600), ("InstallFiles", 4000), ("RegisterProduct", 6100),
        ("PublishProduct", 6200), ("InstallFinalize", 6600),
    ];
    for (action, seq) in &actions {
        pkg.insert_rows(Insert::into("InstallExecuteSequence")
            .row(vec![Value::Str(action.to_string()), Value::Null, Value::Int(*seq)])
        ).unwrap();
    }

    // InstallUISequence
    pkg.create_table("InstallUISequence", vec![
        Column::build("Action").primary_key().string(72),
        Column::build("Condition").nullable().string(255),
        Column::build("Sequence").nullable().int16(),
    ]).unwrap();
    let ui_actions: Vec<(&str, i32)> = vec![
        ("LaunchConditions", 100), ("CostInitialize", 800), ("FileCost", 900),
        ("CostFinalize", 1000), ("ExecuteAction", 1300),
    ];
    for (action, seq) in &ui_actions {
        pkg.insert_rows(Insert::into("InstallUISequence")
            .row(vec![Value::Str(action.to_string()), Value::Null, Value::Int(*seq)])
        ).unwrap();
    }

    // Add cabinet stream
    let cab_data = velocity_msi::build_cabinet(&[velocity_msi::CabinetFile {
        name: "hello.txt".to_string(),
        data: content.to_vec(),
    }]);
    {
        let mut writer = pkg.write_stream("vel.cab").unwrap();
        writer.write_all(&cab_data).unwrap();
    }

    // Get the bytes
    let cursor = pkg.into_inner().unwrap();
    cursor.into_inner()
}

fn create_our_msi() -> Vec<u8> {
    use velocity_msi::{MsiBuilder, Column, Value, CabinetFile, build_cabinet};

    let mut builder = MsiBuilder::new();
    builder.set_title("Velocity Ref Test");
    builder.set_author("Velocity");
    builder.set_template("x64", 1033);

    builder.create_table("Property", vec![
        Column::build("Property").string(72).primary_key().build(),
        Column::build("Value").string(255).nullable().build(),
    ]).unwrap();
    builder.insert_rows("Property", vec![
        vec![Value::from("ProductName"), Value::from("Velocity Ref Test")],
        vec![Value::from("ProductVersion"), Value::from("1.0.0")],
        vec![Value::from("Manufacturer"), Value::from("Velocity Corp")],
        vec![Value::from("ProductCode"), Value::from("{FFFFFFFF-FFFF-FFFF-FFFF-FFFFFFFFFFFF}")],
        vec![Value::from("UpgradeCode"), Value::from("{87654321-4321-4321-4321-CBA987654321}")],
        vec![Value::from("ProductLanguage"), Value::from("1033")],
    ]).unwrap();

    builder.create_table("Directory", vec![
        Column::build("Directory").string(72).primary_key().build(),
        Column::build("Directory_Parent").string(72).nullable().build(),
        Column::build("DefaultDir").string(255).primary_key().build(),
    ]).unwrap();
    builder.insert_rows("Directory", vec![
        vec![Value::from("TARGETDIR"), Value::Null, Value::from("SourceDir")],
        vec![Value::from("INSTALLDIR"), Value::from("TARGETDIR"), Value::from("VelTest:VelTest")],
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
        Column::build("Version").string(72).nullable().build(),
        Column::build("Language").string(20).nullable().build(),
        Column::build("Attributes").int16().nullable().build(),
    ]).unwrap();
    let content = b"Hello World from Velocity Installer!\r\n";
    builder.insert_rows("File", vec![
        vec![Value::from("hello.txt"), Value::from("Comp1"), Value::from("hello.txt"),
             Value::Int(content.len() as i32), Value::Int(1),
             Value::Null, Value::Null, Value::Null],
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
             Value::Null, Value::Null],
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
    builder.insert_rows("Media", vec![
        vec![Value::Int(1), Value::Int(1), Value::Null,
             Value::from("#vel.cab"), Value::Null, Value::Null],
    ]).unwrap();
    builder.add_stream("vel.cab".to_string(), cab);

    builder.create_table("InstallExecuteSequence", vec![
        Column::build("Action").string(72).primary_key().build(),
        Column::build("Condition").string(255).nullable().build(),
        Column::build("Sequence").int16().nullable().build(),
    ]).unwrap();
    builder.insert_rows("InstallExecuteSequence", vec![
        vec![Value::from("LaunchConditions"), Value::Null, Value::Int(100)],
        vec![Value::from("CostInitialize"), Value::Null, Value::Int(800)],
        vec![Value::from("FileCost"), Value::Null, Value::Int(900)],
        vec![Value::from("CostFinalize"), Value::Null, Value::Int(1000)],
        vec![Value::from("InstallValidate"), Value::Null, Value::Int(1400)],
        vec![Value::from("InstallInitialize"), Value::Null, Value::Int(1500)],
        vec![Value::from("ProcessComponents"), Value::Null, Value::Int(1600)],
        vec![Value::from("InstallFiles"), Value::Null, Value::Int(4000)],
        vec![Value::from("RegisterProduct"), Value::Null, Value::Int(6100)],
        vec![Value::from("PublishProduct"), Value::Null, Value::Int(6200)],
        vec![Value::from("InstallFinalize"), Value::Null, Value::Int(6600)],
    ]).unwrap();

    builder.create_table("InstallUISequence", vec![
        Column::build("Action").string(72).primary_key().build(),
        Column::build("Condition").string(255).nullable().build(),
        Column::build("Sequence").int16().nullable().build(),
    ]).unwrap();
    builder.insert_rows("InstallUISequence", vec![
        vec![Value::from("LaunchConditions"), Value::Null, Value::Int(100)],
        vec![Value::from("CostInitialize"), Value::Null, Value::Int(800)],
        vec![Value::from("FileCost"), Value::Null, Value::Int(900)],
        vec![Value::from("CostFinalize"), Value::Null, Value::Int(1000)],
        vec![Value::from("ExecuteAction"), Value::Null, Value::Int(1300)],
    ]).unwrap();

    builder.build().unwrap()
}

fn extract_streams(msi_data: &[u8], prefix: &str) -> std::collections::BTreeMap<String, Vec<u8>> {
    let mut result = std::collections::BTreeMap::new();
    let cursor = std::io::Cursor::new(msi_data);
    let mut comp = cfb::CompoundFile::open(cursor).unwrap();
    let paths: Vec<String> = comp.walk()
        .filter(|e| e.is_stream())
        .map(|e| e.path().to_string_lossy().to_string())
        .collect();
    for path in &paths {
        let mut stream = comp.open_stream(path).unwrap();
        let mut data = Vec::new();
        stream.read_to_end(&mut data).unwrap();
        println!("{}: {} ({} bytes)", prefix, path, data.len());
        result.insert(path.clone(), data);
    }
    result
}
