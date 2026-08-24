/// Diagnostic: create a reference MSI with the msi crate, compare with velocity-msi.
/// cargo run --example compare_with_ref -p velocity-msi
use std::io::{Cursor, Read};

fn read_all_streams<F: std::io::Read + std::io::Seek>(comp: &mut cfb::CompoundFile<F>) -> Vec<(String, Vec<u8>)> {
    let mut result = Vec::new();
    let names: Vec<String> = comp.walk()
        .filter(|e| e.is_stream())
        .map(|e| e.name().to_string())
        .collect();
    for name in &names {
        let mut stream = comp.open_stream(name).unwrap();
        let mut data = Vec::new();
        stream.read_to_end(&mut data).unwrap();
        result.push((name.clone(), data));
    }
    result
}

fn main() {
    println!("=== COMPARE VELOCITY-MSI WITH MSI CRATE REFERENCE ===\n");

    let _ = std::process::Command::new("taskkill")
        .args(&["/F", "/IM", "msiexec.exe"]).output();
    std::thread::sleep(std::time::Duration::from_secs(2));

    // === Build reference MSI using the msi crate ===
    println!("--- Building reference MSI with msi crate ---");
    let ref_path = "C:\\temp\\ref_msi.msi";
    {
        let cursor = Cursor::new(Vec::new());
        let mut pkg = msi::Package::create(msi::PackageType::Installer, cursor).unwrap();

        // Property table
        pkg.create_table("Property", vec![
            msi::Column::build("Property").primary_key().string(72),
            msi::Column::build("Value").nullable().localizable().string(255),
        ]).unwrap();
        pkg.insert_rows(msi::Insert::into("Property").rows(vec![
            vec![msi::Value::Str("ProductName".into()), msi::Value::Str("Ref Product".into())],
            vec![msi::Value::Str("ProductCode".into()), msi::Value::Str("{AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE}".into())],
            vec![msi::Value::Str("ProductVersion".into()), msi::Value::Str("1.0.0".into())],
            vec![msi::Value::Str("Manufacturer".into()), msi::Value::Str("Ref Corp".into())],
            vec![msi::Value::Str("ProductLanguage".into()), msi::Value::Str("1033".into())],
            vec![msi::Value::Str("UpgradeCode".into()), msi::Value::Str("{BBBBBBBB-CCCC-DDDD-EEEE-FFFFFFFFFFFF}".into())],
        ])).unwrap();

        // Directory table
        pkg.create_table("Directory", vec![
            msi::Column::build("Directory").primary_key().string(72),
            msi::Column::build("Directory_Parent").nullable().string(72),
            msi::Column::build("DefaultDir").localizable().string(255),
        ]).unwrap();
        pkg.insert_rows(msi::Insert::into("Directory").rows(vec![
            vec![msi::Value::Str("TARGETDIR".into()), msi::Value::Null, msi::Value::Str(".".into())],
            vec![msi::Value::Str("ProgramFilesFolder".into()), msi::Value::Str("TARGETDIR".into()), msi::Value::Str("PFiles".into())],
            vec![msi::Value::Str("INSTALLDIR".into()), msi::Value::Str("ProgramFilesFolder".into()), msi::Value::Str("RefProduct".into())],
        ])).unwrap();

        // Component table
        pkg.create_table("Component", vec![
            msi::Column::build("Component").primary_key().string(72),
            msi::Column::build("ComponentId").nullable().string(38),
            msi::Column::build("Directory_").string(72),
            msi::Column::build("Attributes").nullable().int16(),
            msi::Column::build("Condition").nullable().string(255),
            msi::Column::build("KeyPath").nullable().string(72),
        ]).unwrap();
        pkg.insert_rows(msi::Insert::into("Component").rows(vec![
            vec![
                msi::Value::Str("MainComp".into()),
                msi::Value::Str("{CCCCCCCC-DDDD-EEEE-FFFF-000000000000}".into()),
                msi::Value::Str("INSTALLDIR".into()),
                msi::Value::Int(0),
                msi::Value::Null,
                msi::Value::Str("hello.txt".into()),
            ],
        ])).unwrap();

        // File table
        pkg.create_table("File", vec![
            msi::Column::build("File").primary_key().string(72),
            msi::Column::build("Component_").string(72),
            msi::Column::build("FileName").localizable().string(255),
            msi::Column::build("FileSize").int32(),
            msi::Column::build("Attributes").nullable().int16(),
            msi::Column::build("Sequence").int16(),
        ]).unwrap();
        pkg.insert_rows(msi::Insert::into("File").rows(vec![
            vec![
                msi::Value::Str("hello.txt".into()),
                msi::Value::Str("MainComp".into()),
                msi::Value::Str("hello.txt".into()),
                msi::Value::Int(30),
                msi::Value::Int(0),
                msi::Value::Int(1),
            ],
        ])).unwrap();

        // Feature table
        pkg.create_table("Feature", vec![
            msi::Column::build("Feature").primary_key().string(38),
            msi::Column::build("Feature_Parent").nullable().string(38),
            msi::Column::build("Title").nullable().localizable().string(64),
            msi::Column::build("Description").nullable().localizable().string(255),
            msi::Column::build("Display").nullable().int16(),
            msi::Column::build("Level").int16(),
            msi::Column::build("Directory_").nullable().string(72),
            msi::Column::build("Attributes").nullable().int16(),
        ]).unwrap();
        pkg.insert_rows(msi::Insert::into("Feature").rows(vec![
            vec![
                msi::Value::Str("Complete".into()),
                msi::Value::Null,
                msi::Value::Str("Complete".into()),
                msi::Value::Str("All features".into()),
                msi::Value::Null,
                msi::Value::Int(1),
                msi::Value::Str("INSTALLDIR".into()),
                msi::Value::Null,
            ],
        ])).unwrap();

        // FeatureComponents
        pkg.create_table("FeatureComponents", vec![
            msi::Column::build("Feature_").primary_key().string(38),
            msi::Column::build("Component_").primary_key().string(72),
        ]).unwrap();
        pkg.insert_rows(msi::Insert::into("FeatureComponents").rows(vec![
            vec![msi::Value::Str("Complete".into()), msi::Value::Str("MainComp".into())],
        ])).unwrap();

        // Media
        pkg.create_table("Media", vec![
            msi::Column::build("DiskId").primary_key().int16(),
            msi::Column::build("LastSequence").int16(),
            msi::Column::build("Cabinet").nullable().string(255),
            msi::Column::build("VolumeLabel").nullable().string(32),
            msi::Column::build("Source").nullable().string(72),
        ]).unwrap();
        pkg.insert_rows(msi::Insert::into("Media").rows(vec![
            vec![
                msi::Value::Int(1),
                msi::Value::Int(1),
                msi::Value::Str("#data.cab".into()),
                msi::Value::Null,
                msi::Value::Null,
            ],
        ])).unwrap();

        // InstallExecuteSequence
        pkg.create_table("InstallExecuteSequence", vec![
            msi::Column::build("Action").primary_key().string(72),
            msi::Column::build("Condition").nullable().string(255),
            msi::Column::build("Sequence").nullable().int16(),
        ]).unwrap();
        pkg.insert_rows(msi::Insert::into("InstallExecuteSequence").rows(vec![
            vec![msi::Value::Str("LaunchConditions".into()), msi::Value::Null, msi::Value::Int(100)],
            vec![msi::Value::Str("ValidateProductID".into()), msi::Value::Null, msi::Value::Int(700)],
            vec![msi::Value::Str("CostFinalize".into()), msi::Value::Null, msi::Value::Int(1000)],
            vec![msi::Value::Str("InstallValidate".into()), msi::Value::Null, msi::Value::Int(1400)],
            vec![msi::Value::Str("InstallInitialize".into()), msi::Value::Null, msi::Value::Int(1500)],
            vec![msi::Value::Str("ProcessComponents".into()), msi::Value::Null, msi::Value::Int(1600)],
            vec![msi::Value::Str("UnpublishComponents".into()), msi::Value::Null, msi::Value::Int(1700)],
            vec![msi::Value::Str("UnpublishFeatures".into()), msi::Value::Null, msi::Value::Int(1800)],
            vec![msi::Value::Str("RegisterProduct".into()), msi::Value::Null, msi::Value::Int(5700)],
            vec![msi::Value::Str("PublishFeatures".into()), msi::Value::Null, msi::Value::Int(6300)],
            vec![msi::Value::Str("PublishProduct".into()), msi::Value::Null, msi::Value::Int(6400)],
            vec![msi::Value::Str("InstallFinalize".into()), msi::Value::Null, msi::Value::Int(6600)],
        ])).unwrap();

        // InstallUISequence
        pkg.create_table("InstallUISequence", vec![
            msi::Column::build("Action").primary_key().string(72),
            msi::Column::build("Condition").nullable().string(255),
            msi::Column::build("Sequence").nullable().int16(),
        ]).unwrap();
        pkg.insert_rows(msi::Insert::into("InstallUISequence").rows(vec![
            vec![msi::Value::Str("LaunchConditions".into()), msi::Value::Null, msi::Value::Int(100)],
            vec![msi::Value::Str("ValidateProductID".into()), msi::Value::Null, msi::Value::Int(700)],
            vec![msi::Value::Str("CostFinalize".into()), msi::Value::Null, msi::Value::Int(1000)],
        ])).unwrap();

        // Summary info
        pkg.summary_info_mut().set_title("Ref Product");
        pkg.summary_info_mut().set_author("Ref Corp");
        pkg.summary_info_mut().set_subject("Test");
        pkg.summary_info_mut().set_arch("x64");
        pkg.summary_info_mut().set_languages(&[msi::Language::from_code(1033)]);
        pkg.summary_info_mut().set_uuid(uuid::Uuid::parse_str("AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE").unwrap());
        pkg.summary_info_mut().set_creating_application("Ref Installer");
        pkg.summary_info_mut().set_word_count(2);
        pkg.summary_info_mut().set_creation_time_to_now();

        pkg.flush().unwrap();
        let cursor = pkg.into_inner().unwrap();
        let ref_data = cursor.into_inner();
        std::fs::write(ref_path, &ref_data).unwrap();
        println!("Reference MSI: {} bytes", ref_data.len());
    }

    // Test reference with msiexec
    println!("\n--- Testing reference MSI with msiexec ---");
    let _ = std::process::Command::new("msiexec")
        .args(&["/x", "{AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE}", "/qn"]).output();
    std::thread::sleep(std::time::Duration::from_secs(1));

    let output = std::process::Command::new("msiexec")
        .args(&["/i", ref_path, "/qn", "/l*v", "C:\\temp\\ref_msi.log"])
        .output().unwrap();
    let ref_exit = output.status.code().unwrap_or(-1);
    println!("Reference MSI exit code: {}", ref_exit);

    // === Build velocity-msi output ===
    println!("\n--- Building velocity-msi output ---");
    let velo_path = "C:\\temp\\velo_compare.msi";
    {
        let mut builder = velocity_msi::MsiBuilder::new();
        builder.set_title("Ref Product");
        builder.set_author("Ref Corp");
        builder.set_subject("Test");
        builder.set_template("x64", 1033);

        builder.create_table("Property", vec![
            velocity_msi::Column::build("Property").string(72).primary_key().build(),
            velocity_msi::Column::build("Value").string(255).nullable().localizable().build(),
        ]).unwrap();
        builder.insert_rows("Property", vec![
            vec![velocity_msi::Value::from("ProductName"), velocity_msi::Value::from("Ref Product")],
            vec![velocity_msi::Value::from("ProductCode"), velocity_msi::Value::from("{AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE}")],
            vec![velocity_msi::Value::from("ProductVersion"), velocity_msi::Value::from("1.0.0")],
            vec![velocity_msi::Value::from("Manufacturer"), velocity_msi::Value::from("Ref Corp")],
            vec![velocity_msi::Value::from("ProductLanguage"), velocity_msi::Value::from("1033")],
            vec![velocity_msi::Value::from("UpgradeCode"), velocity_msi::Value::from("{BBBBBBBB-CCCC-DDDD-EEEE-FFFFFFFFFFFF}")],
        ]).unwrap();

        builder.create_table("Directory", vec![
            velocity_msi::Column::build("Directory").string(72).primary_key().build(),
            velocity_msi::Column::build("Directory_Parent").string(72).nullable().build(),
            velocity_msi::Column::build("DefaultDir").string(255).localizable().build(),
        ]).unwrap();
        builder.insert_rows("Directory", vec![
            vec![velocity_msi::Value::from("TARGETDIR"), velocity_msi::Value::Null, velocity_msi::Value::from(".")],
            vec![velocity_msi::Value::from("ProgramFilesFolder"), velocity_msi::Value::from("TARGETDIR"), velocity_msi::Value::from("PFiles")],
            vec![velocity_msi::Value::from("INSTALLDIR"), velocity_msi::Value::from("ProgramFilesFolder"), velocity_msi::Value::from("RefProduct")],
        ]).unwrap();

        builder.create_table("Component", vec![
            velocity_msi::Column::build("Component").string(72).primary_key().build(),
            velocity_msi::Column::build("ComponentId").string(38).nullable().build(),
            velocity_msi::Column::build("Directory_").string(72).build(),
            velocity_msi::Column::build("Attributes").int16().nullable().build(),
            velocity_msi::Column::build("Condition").string(255).nullable().build(),
            velocity_msi::Column::build("KeyPath").string(72).nullable().build(),
        ]).unwrap();
        builder.insert_rows("Component", vec![
            vec![
                velocity_msi::Value::from("MainComp"),
                velocity_msi::Value::from("{CCCCCCCC-DDDD-EEEE-FFFF-000000000000}"),
                velocity_msi::Value::from("INSTALLDIR"),
                velocity_msi::Value::Int(0),
                velocity_msi::Value::Null,
                velocity_msi::Value::from("hello.txt"),
            ],
        ]).unwrap();

        builder.create_table("File", vec![
            velocity_msi::Column::build("File").string(72).primary_key().build(),
            velocity_msi::Column::build("Component_").string(72).build(),
            velocity_msi::Column::build("FileName").string(255).localizable().build(),
            velocity_msi::Column::build("FileSize").int32().build(),
            velocity_msi::Column::build("Attributes").int16().nullable().build(),
            velocity_msi::Column::build("Sequence").int16().build(),
        ]).unwrap();
        builder.insert_rows("File", vec![
            vec![
                velocity_msi::Value::from("hello.txt"),
                velocity_msi::Value::from("MainComp"),
                velocity_msi::Value::from("hello.txt"),
                velocity_msi::Value::Int(30),
                velocity_msi::Value::Int(0),
                velocity_msi::Value::Int(1),
            ],
        ]).unwrap();

        builder.create_table("Feature", vec![
            velocity_msi::Column::build("Feature").string(38).primary_key().build(),
            velocity_msi::Column::build("Feature_Parent").string(38).nullable().build(),
            velocity_msi::Column::build("Title").string(64).nullable().localizable().build(),
            velocity_msi::Column::build("Description").string(255).nullable().localizable().build(),
            velocity_msi::Column::build("Display").int16().nullable().build(),
            velocity_msi::Column::build("Level").int16().build(),
            velocity_msi::Column::build("Directory_").string(72).nullable().build(),
            velocity_msi::Column::build("Attributes").int16().nullable().build(),
        ]).unwrap();
        builder.insert_rows("Feature", vec![
            vec![
                velocity_msi::Value::from("Complete"),
                velocity_msi::Value::Null,
                velocity_msi::Value::from("Complete"),
                velocity_msi::Value::from("All features"),
                velocity_msi::Value::Null,
                velocity_msi::Value::Int(1),
                velocity_msi::Value::from("INSTALLDIR"),
                velocity_msi::Value::Null,
            ],
        ]).unwrap();

        builder.create_table("FeatureComponents", vec![
            velocity_msi::Column::build("Feature_").string(38).primary_key().build(),
            velocity_msi::Column::build("Component_").string(72).primary_key().build(),
        ]).unwrap();
        builder.insert_rows("FeatureComponents", vec![
            vec![velocity_msi::Value::from("Complete"), velocity_msi::Value::from("MainComp")],
        ]).unwrap();

        builder.create_table("Media", vec![
            velocity_msi::Column::build("DiskId").int16().primary_key().build(),
            velocity_msi::Column::build("LastSequence").int16().build(),
            velocity_msi::Column::build("Cabinet").string(255).nullable().build(),
            velocity_msi::Column::build("VolumeLabel").string(32).nullable().build(),
            velocity_msi::Column::build("Source").string(72).nullable().build(),
        ]).unwrap();
        builder.insert_rows("Media", vec![
            vec![
                velocity_msi::Value::Int(1),
                velocity_msi::Value::Int(1),
                velocity_msi::Value::from("#data.cab"),
                velocity_msi::Value::Null,
                velocity_msi::Value::Null,
            ],
        ]).unwrap();

        builder.create_table("InstallExecuteSequence", vec![
            velocity_msi::Column::build("Action").string(72).primary_key().build(),
            velocity_msi::Column::build("Condition").string(255).nullable().build(),
            velocity_msi::Column::build("Sequence").int16().nullable().build(),
        ]).unwrap();
        builder.insert_rows("InstallExecuteSequence", vec![
            vec![velocity_msi::Value::from("LaunchConditions"), velocity_msi::Value::Null, velocity_msi::Value::Int(100)],
            vec![velocity_msi::Value::from("ValidateProductID"), velocity_msi::Value::Null, velocity_msi::Value::Int(700)],
            vec![velocity_msi::Value::from("CostFinalize"), velocity_msi::Value::Null, velocity_msi::Value::Int(1000)],
            vec![velocity_msi::Value::from("InstallValidate"), velocity_msi::Value::Null, velocity_msi::Value::Int(1400)],
            vec![velocity_msi::Value::from("InstallInitialize"), velocity_msi::Value::Null, velocity_msi::Value::Int(1500)],
            vec![velocity_msi::Value::from("ProcessComponents"), velocity_msi::Value::Null, velocity_msi::Value::Int(1600)],
            vec![velocity_msi::Value::from("UnpublishComponents"), velocity_msi::Value::Null, velocity_msi::Value::Int(1700)],
            vec![velocity_msi::Value::from("UnpublishFeatures"), velocity_msi::Value::Null, velocity_msi::Value::Int(1800)],
            vec![velocity_msi::Value::from("RegisterProduct"), velocity_msi::Value::Null, velocity_msi::Value::Int(5700)],
            vec![velocity_msi::Value::from("PublishFeatures"), velocity_msi::Value::Null, velocity_msi::Value::Int(6300)],
            vec![velocity_msi::Value::from("PublishProduct"), velocity_msi::Value::Null, velocity_msi::Value::Int(6400)],
            vec![velocity_msi::Value::from("InstallFinalize"), velocity_msi::Value::Null, velocity_msi::Value::Int(6600)],
        ]).unwrap();

        builder.create_table("InstallUISequence", vec![
            velocity_msi::Column::build("Action").string(72).primary_key().build(),
            velocity_msi::Column::build("Condition").string(255).nullable().build(),
            velocity_msi::Column::build("Sequence").int16().nullable().build(),
        ]).unwrap();
        builder.insert_rows("InstallUISequence", vec![
            vec![velocity_msi::Value::from("LaunchConditions"), velocity_msi::Value::Null, velocity_msi::Value::Int(100)],
            vec![velocity_msi::Value::from("ValidateProductID"), velocity_msi::Value::Null, velocity_msi::Value::Int(700)],
            vec![velocity_msi::Value::from("CostFinalize"), velocity_msi::Value::Null, velocity_msi::Value::Int(1000)],
        ]).unwrap();

        let msi_data = builder.build().unwrap();
        std::fs::write(velo_path, &msi_data).unwrap();
        println!("Velocity MSI: {} bytes", msi_data.len());
    }

    // === Compare stream by stream ===
    println!("\n=== STREAM-BY-STREAM COMPARISON ===");
    let ref_bytes = std::fs::read(ref_path).unwrap();
    let velo_bytes = std::fs::read(velo_path).unwrap();

    let mut ref_comp = cfb::CompoundFile::open(Cursor::new(&ref_bytes)).unwrap();
    let mut velo_comp = cfb::CompoundFile::open(Cursor::new(&velo_bytes)).unwrap();

    let ref_streams = read_all_streams(&mut ref_comp);
    let velo_streams = read_all_streams(&mut velo_comp);

    println!("Reference streams: {}", ref_streams.len());
    for (name, data) in &ref_streams {
        let safe: String = name.chars().map(|c| if c.is_ascii() && !c.is_control() { c } else { '.' }).collect();
        println!("  {} ({} bytes)", safe, data.len());
    }

    println!("\nVelocity streams: {}", velo_streams.len());
    for (name, data) in &velo_streams {
        let safe: String = name.chars().map(|c| if c.is_ascii() && !c.is_control() { c } else { '.' }).collect();
        println!("  {} ({} bytes)", safe, data.len());
    }

    // Compare matching streams
    println!("\n=== STREAM CONTENT COMPARISON ===");
    for (ref_name, ref_data) in &ref_streams {
        let safe: String = ref_name.chars().map(|c| if c.is_ascii() && !c.is_control() { c } else { '.' }).collect();
        if let Some((_, velo_data)) = velo_streams.iter().find(|(n, _)| n == ref_name) {
            if ref_data == velo_data {
                println!("  {} - IDENTICAL ({} bytes)", safe, ref_data.len());
            } else {
                println!("  {} - DIFFERENT! ref={} velo={} bytes", safe, ref_data.len(), velo_data.len());
                let min_len = ref_data.len().min(velo_data.len());
                for i in 0..min_len {
                    if ref_data[i] != velo_data[i] {
                        println!("    First diff at byte {}: ref=0x{:02X} velo=0x{:02X}", i, ref_data[i], velo_data[i]);
                        let start = i.saturating_sub(4);
                        let end = (i + 32).min(min_len);
                        print!("    ref:  ");
                        for b in &ref_data[start..end] { print!("{:02X} ", b); }
                        println!();
                        print!("    velo: ");
                        for b in &velo_data[start..end] { print!("{:02X} ", b); }
                        println!();
                        break;
                    }
                }
            }
        } else {
            println!("  {} - MISSING in velocity (ref {} bytes)", safe, ref_data.len());
        }
    }

    // Extra streams in velocity
    for (velo_name, velo_data) in &velo_streams {
        if !ref_streams.iter().any(|(n, _)| n == velo_name) {
            let safe: String = velo_name.chars().map(|c| if c.is_ascii() && !c.is_control() { c } else { '.' }).collect();
            println!("  {} - EXTRA in velocity ({} bytes)", safe, velo_data.len());
        }
    }

    // Try opening both with msi crate
    println!("\n=== OPEN WITH MSI CRATE ===");
    match msi::Package::open(Cursor::new(&ref_bytes)) {
        Ok(pkg) => {
            println!("Reference: opened OK");
            for table in pkg.tables() {
                println!("  Table: {} ({} cols)", table.name(), table.columns().len());
            }
        }
        Err(e) => println!("Reference: FAILED: {:?}", e),
    }

    match msi::Package::open(Cursor::new(&velo_bytes)) {
        Ok(pkg) => {
            println!("\nVelocity: opened OK");
            for table in pkg.tables() {
                println!("  Table: {} ({} cols)", table.name(), table.columns().len());
            }
        }
        Err(e) => println!("\nVelocity: FAILED: {:?}", e),
    }

    println!("\n=== DONE ===");
}
