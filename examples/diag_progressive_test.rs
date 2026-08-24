/// Definitive diagnostic: progressive MSI build test
/// Tests each table combination to isolate exactly what causes failures.
///
/// cargo run --example diag_progressive_test -p velocity-msi
use std::io::Cursor;
use velocity_msi::{Column, MsiBuilder, Value, CabinetFile, build_cabinet};

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

fn build_base_msi(pc: &str, uc: &str) -> MsiBuilder {
    let mut builder = MsiBuilder::new();
    builder.set_title("Progressive Test");
    builder.set_author("Velocity");
    builder.set_template("Intel", 1033);

    // Property
    builder.create_table("Property", vec![
        Column::build("Property").string(72).primary_key().build(),
        Column::build("Value").nullable().string(255).localizable().build(),
    ]).unwrap();
    builder.insert_rows("Property", vec![
        vec![Value::from("ProductName"), Value::from("Progressive Test")],
        vec![Value::from("ProductVersion"), Value::from("1.0.0")],
        vec![Value::from("Manufacturer"), Value::from("Velocity")],
        vec![Value::from("ProductCode"), Value::from(pc)],
        vec![Value::from("UpgradeCode"), Value::from(uc)],
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
        vec![Value::from("INSTALLDIR"), Value::from("ProgramFilesFolder"), Value::from("VelTest")],
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
        vec![Value::from("MainFeat"), Value::Null, Value::from("TestFeature"),
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

    builder
}

fn test_msi(name: &str, msi_data: &[u8]) -> i32 {
    let _ = std::fs::create_dir_all("C:\\temp");
    let path = format!("C:\\temp\\{}.msi", name);
    let log_path = format!("C:\\temp\\{}.log", name);
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&log_path);
    std::fs::write(&path, msi_data).unwrap();

    let output = std::process::Command::new("msiexec")
        .args(&["/i", &path, "/qn", "/l*v", &log_path])
        .output().unwrap();
    let ec = output.status.code().unwrap_or(-1);

    // If failed, read log for key errors
    if ec != 0 {
        if let Ok(log) = std::fs::read_to_string(&log_path) {
            let errors: Vec<&str> = log.lines()
                .filter(|l| l.contains("error") || l.contains("Error") ||
                            l.contains("return value 3") || l.contains("Note: 1:"))
                .take(10)
                .collect();
            if !errors.is_empty() {
                println!("  Key log entries:");
                for e in errors {
                    println!("    {}", e.trim());
                }
            }
        }
    }

    // Cleanup
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&log_path);

    ec
}

fn list_streams(name: &str, msi_data: &[u8]) {
    let cursor = Cursor::new(msi_data);
    let comp = cfb::CompoundFile::open(cursor).unwrap();
    let streams: Vec<(String, u64)> = comp.walk()
        .filter(|e| e.is_stream())
        .map(|e| (e.name().to_string(), e.len()))
        .collect();
    println!("  Streams ({}):", streams.len());
    for (sname, size) in &streams {
        // Show first few chars of stream name for debugging
        let display: Vec<u16> = sname.encode_utf16().collect();
        println!("    '{}' ({} bytes) utf16={:?}", sname, size, display);
    }
}

fn main() {
    println!("=== PROGRESSIVE MSI DIAGNOSTIC TEST ===\n");

    let pc = make_uuid();
    let uc = make_uuid();
    let file_content = b"Hello from velocity-msi!\n";

    // ===== TEST 1: Base MSI (no File/Media tables) =====
    println!("--- TEST 1: Base MSI (no File/Media) ---");
    {
        let mut builder = build_base_msi(&pc, &uc);
        let msi_data = builder.build().unwrap();
        println!("  MSI size: {} bytes", msi_data.len());
        list_streams("test1_base", &msi_data);
        let ec = test_msi("test1_base", &msi_data);
        println!("  Exit code: {} {}", ec, if ec == 0 { "OK" } else { "FAIL" });
    }

    // ===== TEST 2: Base + File table (no Media) =====
    println!("\n--- TEST 2: Base + File table (no Media) ---");
    {
        let mut builder = build_base_msi(&pc, &uc);
        builder.create_table("File", vec![
            Column::build("File_").string(72).primary_key().build(),
            Column::build("Component_").string(72).build(),
            Column::build("FileName").string(255).localizable().build(),
            Column::build("FileSize").int32().build(),
            Column::build("Attributes").nullable().int16().build(),
            Column::build("Sequence").int16().build(),
        ]).unwrap();
        builder.insert_rows("File", vec![
            vec![Value::from("MainFile"), Value::from("MainComp"),
                 Value::from("testfile.txt"), Value::Int(file_content.len() as i32),
                 Value::Int(0), Value::Int(1)],
        ]).unwrap();

        let msi_data = builder.build().unwrap();
        println!("  MSI size: {} bytes", msi_data.len());
        list_streams("test2_file_only", &msi_data);
        let ec = test_msi("test2_file_only", &msi_data);
        println!("  Exit code: {} {}", ec, if ec == 0 { "OK" } else { "FAIL" });
    }

    // ===== TEST 3: Base + File + Media with EMBEDDED cabinet (our builder) =====
    println!("\n--- TEST 3: Base + File + Media (embedded cabinet, our builder) ---");
    {
        let mut builder = build_base_msi(&pc, &uc);

        // File table (standard 6-column schema)
        builder.create_table("File", vec![
            Column::build("File_").string(72).primary_key().build(),
            Column::build("Component_").string(72).build(),
            Column::build("FileName").string(255).localizable().build(),
            Column::build("FileSize").int32().build(),
            Column::build("Attributes").nullable().int16().build(),
            Column::build("Sequence").int16().build(),
        ]).unwrap();
        builder.insert_rows("File", vec![
            vec![Value::from("MainFile"), Value::from("MainComp"),
                 Value::from("testfile.txt"), Value::Int(file_content.len() as i32),
                 Value::Int(0), Value::Int(1)],
        ]).unwrap();

        // Media table
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
                 Value::from("#velcab"), Value::Null, Value::Null],
        ]).unwrap();

        // Build cabinet using our builder
        let cab_files = vec![
            CabinetFile {
                name: "testfile.txt".to_string(),
                data: file_content.to_vec(),
            },
        ];
        let cab_data = build_cabinet(&cab_files);
        println!("  Cabinet size: {} bytes", cab_data.len());

        // Add cabinet stream (name without # - the # in Media means "embedded")
        builder.add_stream("velcab".to_string(), cab_data);

        let msi_data = builder.build().unwrap();
        println!("  MSI size: {} bytes", msi_data.len());
        list_streams("test3_embed_our_cab", &msi_data);
        let ec = test_msi("test3_embed_our_cab", &msi_data);
        println!("  Exit code: {} {}", ec, if ec == 0 { "OK" } else { "FAIL" });
    }

    // ===== TEST 4: Base + File + Media with EXTERNAL cabinet reference =====
    println!("\n--- TEST 4: Base + File + Media (external cabinet) ---");
    {
        let mut builder = build_base_msi(&pc, &uc);

        // File table (standard 6-column schema)
        builder.create_table("File", vec![
            Column::build("File_").string(72).primary_key().build(),
            Column::build("Component_").string(72).build(),
            Column::build("FileName").string(255).localizable().build(),
            Column::build("FileSize").int32().build(),
            Column::build("Attributes").nullable().int16().build(),
            Column::build("Sequence").int16().build(),
        ]).unwrap();
        builder.insert_rows("File", vec![
            vec![Value::from("MainFile"), Value::from("MainComp"),
                 Value::from("testfile.txt"), Value::Int(file_content.len() as i32),
                 Value::Int(0), Value::Int(1)],
        ]).unwrap();

        // Media table - external cabinet (no # prefix)
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
                 Value::from("vel.cab"), Value::Null, Value::Null],
        ]).unwrap();

        // Build cabinet and write alongside MSI
        let cab_files = vec![
            CabinetFile {
                name: "testfile.txt".to_string(),
                data: file_content.to_vec(),
            },
        ];
        let cab_data = build_cabinet(&cab_files);
        let _ = std::fs::create_dir_all("C:\\temp");
        std::fs::write("C:\\temp\\vel.cab", &cab_data).unwrap();

        let msi_data = builder.build().unwrap();
        println!("  MSI size: {} bytes", msi_data.len());
        println!("  Cabinet: C:\\temp\\vel.cab ({} bytes)", cab_data.len());
        list_streams("test4_external_cab", &msi_data);
        let ec = test_msi("test4_external_cab", &msi_data);
        println!("  Exit code: {} {}", ec, if ec == 0 { "OK" } else { "FAIL" });

        // Cleanup
        let _ = std::fs::remove_file("C:\\temp\\vel.cab");
    }

    // ===== TEST 5: Same as test 3 but using makecab.exe for known-good cabinet =====
    println!("\n--- TEST 5: Base + File + Media (embedded makecab cabinet) ---");
    {
        let mut builder = build_base_msi(&pc, &uc);

        // File table (standard 6-column schema)
        builder.create_table("File", vec![
            Column::build("File_").string(72).primary_key().build(),
            Column::build("Component_").string(72).build(),
            Column::build("FileName").string(255).localizable().build(),
            Column::build("FileSize").int32().build(),
            Column::build("Attributes").nullable().int16().build(),
            Column::build("Sequence").int16().build(),
        ]).unwrap();
        builder.insert_rows("File", vec![
            vec![Value::from("MainFile"), Value::from("MainComp"),
                 Value::from("testfile.txt"), Value::Int(file_content.len() as i32),
                 Value::Int(0), Value::Int(1)],
        ]).unwrap();

        // Media table
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
                 Value::from("#makecab"), Value::Null, Value::Null],
        ]).unwrap();

        // Try to use makecab.exe to create a known-good cabinet
        let _ = std::fs::create_dir_all("C:\\temp");
        std::fs::write("C:\\temp\\testfile.txt", file_content).unwrap();
        std::fs::write("C:\\temp\\makecab.ddf", 
            ".Option Explicit\n.Set CabinetNameTemplate=makecab.cab\n.Set DiskDirectoryTemplate=C:\\temp\n.Set MaxDiskSize=0\n.Set Cabinet=on\n.Set Compress=on\n\"C:\\temp\\testfile.txt\"\n").unwrap();
        
        let makecab_result = std::process::Command::new("makecab")
            .args(&["/f", "C:\\temp\\makecab.ddf"])
            .output();
        
        let cab_data = match makecab_result {
            Ok(output) if output.status.success() => {
                match std::fs::read("C:\\temp\\makecab.cab") {
                    Ok(data) => {
                        println!("  makecab cabinet: {} bytes", data.len());
                        Some(data)
                    }
                    Err(_) => None,
                }
            }
            _ => None,
        };

        if let Some(cab) = &cab_data {
            builder.add_stream("makecab".to_string(), cab.clone());
            let msi_data = builder.build().unwrap();
            println!("  MSI size: {} bytes", msi_data.len());
            list_streams("test5_makecab", &msi_data);
            let ec = test_msi("test5_makecab", &msi_data);
            println!("  Exit code: {} {}", ec, if ec == 0 { "OK" } else { "FAIL" });
        } else {
            println!("  SKIPPED: makecab.exe not available");
        }

        // Cleanup
        let _ = std::fs::remove_file("C:\\temp\\testfile.txt");
        let _ = std::fs::remove_file("C:\\temp\\makecab.ddf");
        let _ = std::fs::remove_file("C:\\temp\\makecab.cab");
        let _ = std::fs::remove_file("C:\\temp\\setup.inf");
        let _ = std::fs::remove_file("C:\\temp\\setup.rpt");
    }

    println!("\n=== DIAGNOSTIC COMPLETE ===");
    println!("If Test 1 passes but Test 3/5 fails: cabinet embedding is broken");
    println!("If Test 2 passes but Test 3 fails: Media table or stream is broken");
    println!("If Test 4 passes but Test 3 fails: our cabinet format is broken");
    println!("If Test 4 also fails: Media table structure is broken");
}
