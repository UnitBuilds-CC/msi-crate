/// Complete MSI with all required installation tables + embedded cabinet
/// Tests whether the File table needs supporting tables (Dir/Comp/Feature/Media)
/// cargo run --example diag_complete_install -p velocity-msi
use velocity_msi::{CabinetFile, Column, MsiBuilder, Value, build_cabinet};

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

fn main() {
    let product_code = make_uuid();
    let upgrade_code = make_uuid();
    let component_code = make_uuid();

    // File content to install
    let file_content = b"Hello from velocity-msi installer!\r\n";
    let file_name = "velocity_test.txt";

    // Build cabinet with the file
    let cabinet = build_cabinet(&[
        CabinetFile {
            name: file_name.to_string(),
            data: file_content.to_vec(),
        },
    ]);

    println!("Cabinet size: {} bytes", cabinet.len());

    // === TEST 1: Complete MSI with all tables + embedded cabinet ===
    println!("\n=== TEST 1: Complete MSI with all install tables ===");
    {
        let mut b = MsiBuilder::new();
        b.set_title("Velocity Test Product");
        b.set_author("Velocity");
        b.set_template("Intel", 1033);

        // Property table
        b.create_table("Property", vec![
            Column::build("Property").string(72).primary_key().build(),
            Column::build("Value").nullable().string(255).localizable().build(),
        ]).unwrap();
        b.insert_rows("Property", vec![
            vec![Value::from("ProductName"), Value::from("Velocity Test")],
            vec![Value::from("ProductVersion"), Value::from("1.0.0")],
            vec![Value::from("Manufacturer"), Value::from("Velocity")],
            vec![Value::from("ProductCode"), Value::from(product_code.as_str())],
            vec![Value::from("UpgradeCode"), Value::from(upgrade_code.as_str())],
            vec![Value::from("ProductLanguage"), Value::from("1033")],
        ]).unwrap();

        // Directory table
        // TARGETDIR is the implicit root; INSTALLDIR is where files go
        b.create_table("Directory", vec![
            Column::build("Directory").string(72).primary_key().build(),
            Column::build("Directory_Parent").nullable().string(72).build(),
            Column::build("DefaultDir").string(255).localizable().build(),
        ]).unwrap();
        b.insert_rows("Directory", vec![
            vec![Value::from("INSTALLDIR"), Value::from("TARGETDIR"), Value::from(".")],
            vec![Value::from("TARGETDIR"), Value::Null, Value::from("C:\\Program Files\\VelocityTest")],
        ]).unwrap();

        // Component table
        b.create_table("Component", vec![
            Column::build("Component").string(72).primary_key().build(),
            Column::build("ComponentId").nullable().string(38).build(),
            Column::build("Directory_").string(72).build(),
            Column::build("Attributes").int16().build(),
            Column::build("Condition").nullable().string(255).build(),
            Column::build("KeyPath").nullable().string(72).build(),
        ]).unwrap();
        b.insert_rows("Component", vec![
            vec![
                Value::from("MainComponent"),
                Value::from(component_code.as_str()),
                Value::from("INSTALLDIR"),
                Value::Int(0),    // no special attributes
                Value::Null,      // no condition
                Value::from("MainFile"),  // KeyPath = File entry
            ],
        ]).unwrap();

        // Feature table
        b.create_table("Feature", vec![
            Column::build("Feature").string(38).primary_key().build(),
            Column::build("Feature_Parent").nullable().string(38).build(),
            Column::build("Title").nullable().string(64).localizable().build(),
            Column::build("Description").nullable().string(255).localizable().build(),
            Column::build("Display").nullable().int16().build(),
            Column::build("Level").int16().build(),
            Column::build("Directory_").nullable().string(72).build(),
            Column::build("Attributes").int16().build(),
        ]).unwrap();
        b.insert_rows("Feature", vec![
            vec![
                Value::from("Complete"),    // Feature name
                Value::Null,                // No parent
                Value::from("Complete"),    // Title
                Value::from("Full install"),// Description
                Value::Int(2),              // Display
                Value::Int(1),              // Level (1 = default install)
                Value::from("INSTALLDIR"),  // Directory
                Value::Int(0),              // Attributes
            ],
        ]).unwrap();

        // FeatureComponents table
        b.create_table("FeatureComponents", vec![
            Column::build("Feature_").string(38).primary_key().build(),
            Column::build("Component_").string(72).primary_key().build(),
        ]).unwrap();
        b.insert_rows("FeatureComponents", vec![
            vec![Value::from("Complete"), Value::from("MainComponent")],
        ]).unwrap();

        // File table
        b.create_table("File", vec![
            Column::build("File_").string(72).primary_key().build(),
            Column::build("Component_").string(72).build(),
            Column::build("FileName").string(255).localizable().build(),
            Column::build("FileSize").int32().build(),
            Column::build("Attributes").nullable().int16().build(),
            Column::build("Sequence").int16().build(),
        ]).unwrap();
        b.insert_rows("File", vec![
            vec![
                Value::from("MainFile"),
                Value::from("MainComponent"),
                Value::from(file_name),
                Value::Int(file_content.len() as i32),
                Value::Int(0),      // no attributes
                Value::Int(1),      // sequence
            ],
        ]).unwrap();

        // Media table
        b.create_table("Media", vec![
            Column::build("DiskId").int16().primary_key().build(),
            Column::build("LastSequence").int16().build(),
            Column::build("Cabinet").nullable().string(255).build(),
            Column::build("VolumeLabel").nullable().string(32).build(),
            Column::build("Source").nullable().string(72).build(),
        ]).unwrap();
        b.insert_rows("Media", vec![
            vec![
                Value::Int(1),
                Value::Int(1),              // LastSequence = 1 (one file)
                Value::from("#velocity.cab"), // Embedded cabinet (# prefix = embedded)
                Value::Null,                 // No volume label
                Value::Null,                 // No source
            ],
        ]).unwrap();

        // Embed the cabinet as a stream
        b.add_stream("velocity.cab".to_string(), cabinet.clone());

        // Build the MSI
        let msi_data = b.build().unwrap();
        println!("MSI size: {} bytes", msi_data.len());

        // Write and test with msiexec
        let _ = std::fs::create_dir_all("C:\\temp");
        let path = "C:\\temp\\test_complete_install.msi";
        let log_path = "C:\\temp\\test_complete_install.log";
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(log_path);
        std::fs::write(path, &msi_data).unwrap();

        let output = std::process::Command::new("msiexec")
            .args(&["/i", path, "/qn", "/l*v", log_path])
            .output().unwrap();
        let ec = output.status.code().unwrap_or(-1);
        println!("Exit code: {}", ec);

        if ec != 0 {
            // Print relevant log lines
            if let Ok(log) = std::fs::read_to_string(log_path) {
                let lines: Vec<&str> = log.lines().collect();
                let start = lines.len().saturating_sub(80);
                println!("\n--- Last 80 lines of log ---");
                for line in &lines[start..] {
                    println!("{}", line);
                }
            }
        } else {
            println!("\nSUCCESS! MSI installed cleanly!");
            // Check if file was installed
            let install_path = "C:\\Program Files\\VelocityTest\\velocity_test.txt";
            if std::path::Path::new(install_path).exists() {
                println!("File installed at: {}", install_path);
                let content = std::fs::read_to_string(install_path).unwrap();
                println!("Content: {:?}", content);
            } else {
                println!("WARNING: File not found at expected path: {}", install_path);
                // Check what's in the install dir
                if let Ok(entries) = std::fs::read_dir("C:\\Program Files\\VelocityTest") {
                    for entry in entries {
                        if let Ok(entry) = entry {
                            println!("  Found: {:?}", entry.path());
                        }
                    }
                }
            }
        }
    }

    // === TEST 2: Same but without cabinet (no files, just tables) ===
    println!("\n\n=== TEST 2: All tables but NO cabinet/file data ===");
    {
        let mut b = MsiBuilder::new();
        b.set_title("Velocity Test NoFiles");
        b.set_author("Velocity");
        b.set_template("Intel", 1033);

        b.create_table("Property", vec![
            Column::build("Property").string(72).primary_key().build(),
            Column::build("Value").nullable().string(255).localizable().build(),
        ]).unwrap();
        b.insert_rows("Property", vec![
            vec![Value::from("ProductName"), Value::from("Velocity Test NF")],
            vec![Value::from("ProductVersion"), Value::from("1.0.0")],
            vec![Value::from("Manufacturer"), Value::from("Velocity")],
            vec![Value::from("ProductCode"), Value::from(make_uuid().as_str())],
            vec![Value::from("UpgradeCode"), Value::from(make_uuid().as_str())],
            vec![Value::from("ProductLanguage"), Value::from("1033")],
        ]).unwrap();

        b.create_table("Directory", vec![
            Column::build("Directory").string(72).primary_key().build(),
            Column::build("Directory_Parent").nullable().string(72).build(),
            Column::build("DefaultDir").string(255).localizable().build(),
        ]).unwrap();
        b.insert_rows("Directory", vec![
            vec![Value::from("INSTALLDIR"), Value::from("TARGETDIR"), Value::from(".")],
            vec![Value::from("TARGETDIR"), Value::Null, Value::from("C:\\Program Files\\VelocityTestNF")],
        ]).unwrap();

        b.create_table("Component", vec![
            Column::build("Component").string(72).primary_key().build(),
            Column::build("ComponentId").nullable().string(38).build(),
            Column::build("Directory_").string(72).build(),
            Column::build("Attributes").int16().build(),
            Column::build("Condition").nullable().string(255).build(),
            Column::build("KeyPath").nullable().string(72).build(),
        ]).unwrap();
        b.insert_rows("Component", vec![
            vec![
                Value::from("MainComponent"),
                Value::from(make_uuid().as_str()),
                Value::from("INSTALLDIR"),
                Value::Int(0),
                Value::Null,
                Value::Null,
            ],
        ]).unwrap();

        b.create_table("Feature", vec![
            Column::build("Feature").string(38).primary_key().build(),
            Column::build("Feature_Parent").nullable().string(38).build(),
            Column::build("Title").nullable().string(64).localizable().build(),
            Column::build("Description").nullable().string(255).localizable().build(),
            Column::build("Display").nullable().int16().build(),
            Column::build("Level").int16().build(),
            Column::build("Directory_").nullable().string(72).build(),
            Column::build("Attributes").int16().build(),
        ]).unwrap();
        b.insert_rows("Feature", vec![
            vec![
                Value::from("Complete"), Value::Null,
                Value::from("Complete"), Value::from("Full install"),
                Value::Int(2), Value::Int(1),
                Value::from("INSTALLDIR"), Value::Int(0),
            ],
        ]).unwrap();

        b.create_table("FeatureComponents", vec![
            Column::build("Feature_").string(38).primary_key().build(),
            Column::build("Component_").string(72).primary_key().build(),
        ]).unwrap();
        b.insert_rows("FeatureComponents", vec![
            vec![Value::from("Complete"), Value::from("MainComponent")],
        ]).unwrap();

        // NO File table, NO Media table, NO cabinet

        let msi_data = b.build().unwrap();
        println!("MSI size: {} bytes", msi_data.len());

        let _ = std::fs::create_dir_all("C:\\temp");
        let path = "C:\\temp\\test_nofiles.msi";
        let log_path = "C:\\temp\\test_nofiles.log";
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(log_path);
        std::fs::write(path, &msi_data).unwrap();

        let output = std::process::Command::new("msiexec")
            .args(&["/i", path, "/qn", "/l*v", log_path])
            .output().unwrap();
        let ec = output.status.code().unwrap_or(-1);
        println!("Exit code: {}", ec);

        if ec != 0 {
            if let Ok(log) = std::fs::read_to_string(log_path) {
                let lines: Vec<&str> = log.lines().collect();
                let start = lines.len().saturating_sub(40);
                println!("\n--- Last 40 lines of log ---");
                for line in &lines[start..] {
                    println!("{}", line);
                }
            }
        } else {
            println!("SUCCESS! MSI without files installed cleanly!");
        }
    }
}
