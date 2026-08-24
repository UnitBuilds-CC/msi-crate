/// Definitive test: build MSI with velocity-msi, install with msiexec,
/// verify files on disk, uninstall, verify cleanup.
use std::io::Cursor;
use std::path::Path;

fn main() {
    let output_dir = Path::new(r"C:\Users\visse\OneDrive\Documentos\V.E.L.O.C.I.T.Y.-Installer-master\examples\sample-app\output");
    let msi_path = output_dir.join("definitive_test.msi");
    // Per-user install directory (no admin needed)
    let local_app_data = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| r"C:\Users\visse\AppData\Local".to_string());
    let install_dir = format!(r"{}\DefinitiveTest", local_app_data);

    // Create test files
    let test_dir = output_dir.join("test_files");
    std::fs::create_dir_all(&test_dir).unwrap();
    std::fs::write(test_dir.join("hello.txt"), "Hello from velocity-msi!").unwrap();
    std::fs::write(test_dir.join("data.txt"), "Test data file for uninstall verification").unwrap();

    // Build MSI
    let mut builder = velocity_msi::MsiBuilder::new();
    builder.set_title("Definitive Test");
    builder.set_author("Velocity Team");
    builder.set_subject("Definitive Install/Uninstall Test");
    builder.set_template("x64", 1033);
    builder.set_include_validation(false); // Test without _Validation

    // Create tables
    create_tables(&mut builder);

    // Populate Property table
    let product_code = "{AABBCCDD-1234-5678-9ABC-DEF012345678}";
    let upgrade_code = "{11223344-5566-7788-99AA-BBCCDDEEFF00}";
    populate_properties(&mut builder, product_code, upgrade_code);

    // Populate Directory table
    populate_directories(&mut builder, &install_dir);

    // Read test files
    let files = vec![
        (test_dir.join("hello.txt"), "hello.txt"),
        (test_dir.join("data.txt"), "data.txt"),
    ];

    // Populate Component, File, Media tables
    populate_components(&mut builder, &files);

    // Populate Feature + FeatureComponents
    populate_features(&mut builder, files.len());

    // Populate InstallExecuteSequence + InstallUISequence
    populate_sequences(&mut builder);

    // Build cabinet
    let file_ids: Vec<String> = (0..files.len()).map(|i| format!("file_{}", i)).collect();
    build_cabinet(&mut builder, &files, &file_ids);

    // Build MSI
    std::fs::create_dir_all(output_dir).unwrap();
    let msi_data = builder.build().unwrap();
    std::fs::write(&msi_path, &msi_data).unwrap();
    eprintln!("MSI written: {} ({} bytes)", msi_path.display(), msi_data.len());

    // Dump table info for diagnostics
    dump_msi_info(&msi_data);

    eprintln!("\nTo install:");
    eprintln!("  Start-Process msiexec -ArgumentList '/i','{}','/qn','/l*vx','{}' -Wait",
        msi_path.display(), output_dir.join("definitive_install.log").display());
    eprintln!("\nTo check files:");
    eprintln!("  Get-ChildItem '{}' -Recurse", install_dir);
    eprintln!("\nTo uninstall:");
    eprintln!("  Start-Process msiexec -ArgumentList '/x','{}','/qn','/l*vx','{}' -Wait",
        product_code, output_dir.join("definitive_uninstall.log").display());
    eprintln!("\nTo check cleanup:");
    eprintln!("  Test-Path '{}'", install_dir);
}

fn create_tables(builder: &mut velocity_msi::MsiBuilder) {
    use velocity_msi::Column;

    builder.create_table("Property", vec![
        Column::build("Property").string(72).primary_key().build(),
        Column::build("Value").string(255).nullable().build(),
    ]).unwrap();

    builder.create_table("Directory", vec![
        Column::build("Directory").string(72).primary_key().build(),
        Column::build("Directory_Parent").string(72).nullable().build(),
        Column::build("DefaultDir").string(255).nullable().build(),
    ]).unwrap();

    builder.create_table("Component", vec![
        Column::build("Component").string(72).primary_key().build(),
        Column::build("ComponentId").string(38).nullable().build(),
        Column::build("Directory_").string(72).build(),
        Column::build("Attributes").int16().build(),
        Column::build("Condition").string(255).nullable().build(),
        Column::build("KeyPath").string(72).nullable().build(),
    ]).unwrap();

    builder.create_table("File", vec![
        Column::build("File").string(72).primary_key().build(),
        Column::build("Component_").string(72).build(),
        Column::build("FileName").string(255).build(),
        Column::build("FileSize").int32().build(),
        Column::build("Version").string(72).nullable().build(),
        Column::build("Language").string(20).nullable().build(),
        Column::build("Attributes").int16().nullable().build(),
        Column::build("Sequence").int32().build(),
    ]).unwrap();

    builder.create_table("Media", vec![
        Column::build("DiskId").int16().primary_key().build(),
        Column::build("LastSequence").int32().build(),
        Column::build("DiskPrompt").string(255).nullable().build(),
        Column::build("VolumeLabel").string(32).nullable().build(),
        Column::build("Cabinet").string(255).nullable().build(),
        Column::build("Source").string(72).nullable().build(),
    ]).unwrap();

    builder.create_table("Feature", vec![
        Column::build("Feature").string(38).primary_key().build(),
        Column::build("Feature_Parent").string(38).nullable().build(),
        Column::build("Title").string(64).nullable().build(),
        Column::build("Description").string(255).nullable().build(),
        Column::build("Display").int16().nullable().build(),
        Column::build("Level").int16().build(),
        Column::build("Directory_").string(72).nullable().build(),
        Column::build("Attributes").int16().build(),
    ]).unwrap();

    builder.create_table("FeatureComponents", vec![
        Column::build("Feature_").string(38).primary_key().build(),
        Column::build("Component_").string(72).primary_key().build(),
    ]).unwrap();

    builder.create_table("InstallExecuteSequence", vec![
        Column::build("Action").string(72).primary_key().build(),
        Column::build("Condition").string(255).nullable().build(),
        Column::build("Sequence").int16().nullable().build(),
    ]).unwrap();

    builder.create_table("InstallUISequence", vec![
        Column::build("Action").string(72).primary_key().build(),
        Column::build("Condition").string(255).nullable().build(),
        Column::build("Sequence").int16().nullable().build(),
    ]).unwrap();
}

fn populate_properties(builder: &mut velocity_msi::MsiBuilder, product_code: &str, upgrade_code: &str) {
    use velocity_msi::Value;
    let props = vec![
        ("ProductName", "Definitive Test Product"),
        ("ProductVersion", "1.0.0"),
        ("Manufacturer", "Velocity Team"),
        ("ProductCode", product_code),
        ("UpgradeCode", upgrade_code),
        ("ProductLanguage", "1033"),
    ];
    for (name, value) in props {
        builder.insert_rows("Property", vec![vec![Value::from(name), Value::from(value)]]).unwrap();
    }
}

fn populate_directories(builder: &mut velocity_msi::MsiBuilder, _install_dir: &str) {
    use velocity_msi::Value;

    // TARGETDIR root
    builder.insert_rows("Directory", vec![vec![
        Value::from("TARGETDIR"), Value::Null, Value::from("SourceDir"),
    ]]).unwrap();

    // LocalAppDataFolder for per-user install (no admin needed)
    builder.insert_rows("Directory", vec![vec![
        Value::from("LocalAppDataFolder"), Value::from("TARGETDIR"), Value::from("LocalAppData"),
    ]]).unwrap();

    // INSTALLDIR under LocalAppData
    builder.insert_rows("Directory", vec![vec![
        Value::from("INSTALLDIR"),
        Value::from("LocalAppDataFolder"),
        Value::from("DefinitiveTest:DefinitiveTest"),
    ]]).unwrap();
}

fn populate_components(builder: &mut velocity_msi::MsiBuilder, files: &[(std::path::PathBuf, &str)]) {
    use velocity_msi::Value;

    // Generate unique GUIDs for each component
    let component_guids = vec![
        "{12345678-1234-1234-1234-123456789ABC}",
        "{22345678-1234-1234-1234-123456789ABC}",
        "{32345678-1234-1234-1234-123456789ABC}",
        "{42345678-1234-1234-1234-123456789ABC}",
    ];

    for (i, (file_path, file_name)) in files.iter().enumerate() {
        let component_id = format!("comp_{}", i);
        let file_id = format!("file_{}", i);
        let file_size = std::fs::metadata(file_path).map(|m| m.len() as i32).unwrap_or(0);
        let component_guid = component_guids.get(i).unwrap_or(&"{00000000-0000-0000-0000-000000000000}");

        // Component with valid GUID, KeyPath = file
        builder.insert_rows("Component", vec![vec![
            Value::from(component_id.as_str()),
            Value::from(*component_guid),      // ComponentId: valid GUID
            Value::from("INSTALLDIR"),
            Value::Int(0),                     // Attributes
            Value::Null,                       // Condition
            Value::from(file_id.as_str()), // KeyPath
        ]]).unwrap();

        // File row
        builder.insert_rows("File", vec![vec![
            Value::from(file_id.as_str()),
            Value::from(component_id.as_str()),
            Value::from(*file_name),
            Value::Int(file_size),
            Value::Null,   // Version
            Value::Null,   // Language
            Value::Int(0), // Attributes
            Value::Int((i + 1) as i32), // Sequence
        ]]).unwrap();
    }

    // Media table
    builder.insert_rows("Media", vec![vec![
        Value::Int(1),
        Value::Int(files.len() as i32),
        Value::Null,
        Value::Null,
        Value::from("#Velocity.cab"),
        Value::Null,
    ]]).unwrap();
}

fn populate_features(builder: &mut velocity_msi::MsiBuilder, file_count: usize) {
    use velocity_msi::Value;

    // Main feature
    builder.insert_rows("Feature", vec![vec![
        Value::from("Complete"),
        Value::Null,
        Value::from("Complete"),
        Value::from("All files"),
        Value::Int(1),
        Value::Int(1),  // Level = 1 (install by default)
        Value::from("INSTALLDIR"),
        Value::Int(0),
    ]]).unwrap();

    // Link components to feature
    for i in 0..file_count {
        builder.insert_rows("FeatureComponents", vec![vec![
            Value::from("Complete"),
            Value::from(format!("comp_{}", i).as_str()),
        ]]).unwrap();
    }
}

fn populate_sequences(builder: &mut velocity_msi::MsiBuilder) {
    use velocity_msi::Value;

    // Standard execute sequence (correct relative order)
    let actions: Vec<(&str, Option<&str>, i16)> = vec![
        ("LaunchConditions", Some("NOT Installed"), 100),
        ("CostInitialize", None, 800),
        ("FileCost", None, 900),
        ("CostFinalize", None, 1000),
        ("InstallValidate", None, 1400),
        ("InstallInitialize", None, 1500),
        ("ProcessComponents", None, 1600),
        ("RemoveFiles", Some("Installed"), 3500),  // Uninstall: remove old files
        ("InstallFiles", None, 4000),              // Install: copy new files
        ("RegisterProduct", None, 6100),
        ("PublishFeatures", None, 6300),
        ("PublishProduct", None, 6400),
        ("InstallFinalize", None, 6600),
    ];

    for (action, condition, seq) in actions {
        let cond_val = match condition {
            Some(c) => Value::from(c),
            None => Value::Null,
        };
        builder.insert_rows("InstallExecuteSequence", vec![vec![
            Value::from(action), cond_val, Value::Int(seq as i32),
        ]]).unwrap();
    }

    // UI sequence
    let ui_actions: Vec<(&str, Option<&str>, i16)> = vec![
        ("LaunchConditions", None, 100),
        ("CostInitialize", None, 800),
        ("CostFinalize", None, 1000),
        ("ExecuteAction", None, 1300),
    ];

    for (action, condition, seq) in ui_actions {
        let cond_val = match condition {
            Some(c) => Value::from(c),
            None => Value::Null,
        };
        builder.insert_rows("InstallUISequence", vec![vec![
            Value::from(action), cond_val, Value::Int(seq as i32),
        ]]).unwrap();
    }
}

fn build_cabinet(builder: &mut velocity_msi::MsiBuilder, files: &[(std::path::PathBuf, &str)], file_ids: &[String]) {
    let mut cab_data = Cursor::new(Vec::new());
    {
        let mut cab_builder = cab::CabinetBuilder::new();
        let folder = cab_builder.add_folder(cab::CompressionType::MsZip);
        for id in file_ids {
            folder.add_file(id);
        }

        let mut cab_writer = cab_builder.build(&mut cab_data).unwrap();
        for (file_path, _) in files {
            let mut writer = cab_writer.next_file().unwrap().unwrap();
            let mut reader = std::fs::File::open(file_path).unwrap();
            std::io::copy(&mut reader, &mut writer).unwrap();
        }
        cab_writer.finish().unwrap();
    }

    builder.add_stream("Velocity.cab".to_string(), cab_data.into_inner());
}

fn dump_msi_info(msi_data: &[u8]) {
    // Validate OLE structure
    match velocity_msi::validate_ole(msi_data) {
        Ok(info) => {
            eprintln!("\nOLE Validation:");
            eprintln!("  Valid OLE: {}", info.valid_ole);
            eprintln!("  Has Summary: {}", info.has_summary);
            eprintln!("  Has String Pool: {}", info.has_string_pool);
            eprintln!("  Streams ({}):", info.stream_names.len());
            for name in &info.stream_names {
                eprintln!("    {}", name);
            }
        }
        Err(e) => eprintln!("OLE validation failed: {}", e),
    }
}
