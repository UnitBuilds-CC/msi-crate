//! Definitive test: Complete MSI with 8-column File table matching reference MSI schema.
//! 
//! Reference File table (from Office C2RInt.16.msi):
//!   Col 1: "File"       string(72) PK        Type=0x2D48
//!   Col 2: "Component_" string(72)           Type=0x0D48
//!   Col 3: "FileName"   string(255)          Type=0x0FFF
//!   Col 4: "FileSize"   int32                Type=0x0104
//!   Col 5: "Version"    string(72) nullable  Type=0x1D48
//!   Col 6: "Language"   int16 nullable       Type=0x1D14
//!   Col 7: "Attributes" int16 nullable       Type=0x1502
//!   Col 8: "Sequence"   int32                Type=0x0104

use std::process::Command;
use velocity_msi::{MsiBuilder, Column, Value, CabinetFile, build_cabinet};

fn main() {
    println!("=== Definitive test: 8-column File table ===\n");

    let install_dir = "C:\\temp\\veloctiy_file_test";
    std::fs::create_dir_all(install_dir).ok();

    // Create a test file for the cabinet
    let test_content = b"Hello from Velocity MSI!";
    let test_file_name = "velocity_test.txt";

    // Build cabinet with the test file
    // IMPORTANT: CFFILE name must match the File table primary key (SourceCabKey)
    let cabinet = build_cabinet(&[
        CabinetFile {
            name: "F1".to_string(),  // Must match File.File primary key
            data: test_content.to_vec(),
        },
    ]);

    println!("Cabinet size: {} bytes", cabinet.len());

    // Use a writable target directory to avoid admin privilege issues
    let target_dir = "C:\\temp\\velocity_install";
    std::fs::create_dir_all(target_dir).ok();

    // Build the MSI
    let mut b = MsiBuilder::new();
    b.set_title("Velocity File Test");
    b.set_author("Velocity Installer");
    b.set_subject("File Installation Test");
    b.set_comments("Tests 8-column File table schema");
    b.set_template("x64", 1033);

    // === Property table ===
    b.create_table("Property", vec![
        Column::build("Property").string(72).primary_key().build(),
        Column::build("Value").string(255).nullable().build(),
    ]).unwrap();
    b.insert_rows("Property", vec![
        vec![Value::from("ProductName"), Value::from("Velocity File Test")],
        vec![Value::from("ProductCode"), Value::from("{D1E2F3A4-B5C6-7890-ABCD-EF1234567890}")],
        vec![Value::from("ProductVersion"), Value::from("1.0.0")],
        vec![Value::from("ProductLanguage"), Value::from("1033")],
        vec![Value::from("Manufacturer"), Value::from("Velocity")],
        vec![Value::from("UpgradeCode"), Value::from("{E2F3A4B5-C6D7-8901-BCDE-F12345678901}")],
    ]).unwrap();

    // === Directory table ===
    // Use a simple hierarchy: TARGETDIR → VelocityDir
    // Avoid ProgramFilesFolder to prevent permission issues in testing
    b.create_table("Directory", vec![
        Column::build("Directory").string(72).primary_key().build(),
        Column::build("Directory_Parent").string(72).nullable().build(),
        Column::build("DefaultDir").string(255).build(),
    ]).unwrap();
    b.insert_rows("Directory", vec![
        vec![Value::from("TARGETDIR"), Value::Null, Value::from("SourceDir")],
        vec![Value::from("VelocityDir"), Value::from("TARGETDIR"), Value::from("VelocityTest")],
    ]).unwrap();

    // === Component table ===
    b.create_table("Component", vec![
        Column::build("Component").string(72).primary_key().build(),
        Column::build("ComponentId").string(72).nullable().build(),
        Column::build("Directory_").string(72).build(),
        Column::build("Attributes").int16().build(),
        Column::build("Condition").string(255).nullable().build(),
        Column::build("KeyPath").string(72).nullable().build(),
    ]).unwrap();
    b.insert_rows("Component", vec![
        vec![
            Value::from("MainComponent"),
            Value::from("{A1B2C3D4-E5F6-7890-ABCD-EF1234567890}"),
            Value::from("VelocityDir"),
            Value::from(0i32),      // Attributes: 0 = normal
            Value::Null,             // Condition
            Value::from("F1"),       // KeyPath → File table
        ],
    ]).unwrap();

    // === Feature table ===
    b.create_table("Feature", vec![
        Column::build("Feature").string(72).primary_key().build(),
        Column::build("Feature_Parent").string(72).nullable().build(),
        Column::build("Title").string(255).nullable().build(),
        Column::build("Description").string(255).nullable().build(),
        Column::build("Display").int16().nullable().build(),
        Column::build("Level").int16().build(),
        Column::build("Directory_").string(72).nullable().build(),
        Column::build("Attributes").int16().build(),
    ]).unwrap();
    b.insert_rows("Feature", vec![
        vec![
            Value::from("MainFeature"),
            Value::Null,             // No parent
            Value::from("Complete"), // Title
            Value::from("All files"),// Description
            Value::from(1i32),       // Display
            Value::from(1i32),       // Level (1 = install by default)
            Value::Null,             // Directory_
            Value::from(0i32),       // Attributes
        ],
    ]).unwrap();

    // === FeatureComponents table ===
    b.create_table("FeatureComponents", vec![
        Column::build("Feature_").string(72).primary_key().build(),
        Column::build("Component_").string(72).primary_key().build(),
    ]).unwrap();
    b.insert_rows("FeatureComponents", vec![
        vec![Value::from("MainFeature"), Value::from("MainComponent")],
    ]).unwrap();

    // === File table (8 columns matching reference MSI) ===
    b.create_table("File", vec![
        Column::build("File").string(72).primary_key().build(),
        Column::build("Component_").string(72).build(),
        Column::build("FileName").string(255).build(),
        Column::build("FileSize").int32().build(),
        Column::build("Version").string(72).nullable().build(),
        Column::build("Language").int16().nullable().build(),
        Column::build("Attributes").int16().nullable().build(),
        Column::build("Sequence").int32().build(),
    ]).unwrap();
    b.insert_rows("File", vec![
        vec![
            Value::from("F1"),                          // File (primary key)
            Value::from("MainComponent"),               // Component_
            Value::from(test_file_name),                // FileName
            Value::from(test_content.len() as i32),     // FileSize
            Value::Null,                                // Version (nullable)
            Value::Null,                                // Language (nullable)
            Value::from(0i32),                          // Attributes (nullable, 0=normal)
            Value::from(1i32),                          // Sequence
        ],
    ]).unwrap();

    // === Media table (6 columns: DiskId, LastSequence, DiskPrompt, VolumeLabel, Cabinet, Source) ===
    b.create_table("Media", vec![
        Column::build("DiskId").int16().primary_key().build(),
        Column::build("LastSequence").int32().build(),
        Column::build("DiskPrompt").string(255).nullable().build(),
        Column::build("VolumeLabel").string(32).nullable().build(),
        Column::build("Cabinet").string(255).nullable().build(),
        Column::build("Source").string(72).nullable().build(),
    ]).unwrap();
    b.insert_rows("Media", vec![
        vec![
            Value::from(1i32),          // DiskId
            Value::from(1i32),          // LastSequence
            Value::Null,                // DiskPrompt (nullable)
            Value::Null,                // VolumeLabel
            Value::from("#velo.cab"),   // Cabinet (# prefix = embedded)
            Value::Null,                // Source
        ],
    ]).unwrap();

    // === InstallExecuteSequence table (required for install actions) ===
    b.create_table("InstallExecuteSequence", vec![
        Column::build("Action").string(72).primary_key().build(),
        Column::build("Condition").string(255).nullable().build(),
        Column::build("Sequence").int16().nullable().build(),
    ]).unwrap();
    b.insert_rows("InstallExecuteSequence", vec![
        vec![Value::from("CostInitialize"), Value::Null, Value::from(800i32)],
        vec![Value::from("CostFinalize"), Value::Null, Value::from(1000i32)],
        vec![Value::from("InstallValidate"), Value::Null, Value::from(1400i32)],
        vec![Value::from("InstallInitialize"), Value::Null, Value::from(1500i32)],
        vec![Value::from("ProcessComponents"), Value::Null, Value::from(1600i32)],
        vec![Value::from("InstallFiles"), Value::Null, Value::from(4000i32)],
        vec![Value::from("RegisterProduct"), Value::Null, Value::from(5700i32)],
        vec![Value::from("InstallFinalize"), Value::Null, Value::from(6600i32)],
    ]).unwrap();

    // === InstallUISequence table ===
    b.create_table("InstallUISequence", vec![
        Column::build("Action").string(72).primary_key().build(),
        Column::build("Condition").string(255).nullable().build(),
        Column::build("Sequence").int16().nullable().build(),
    ]).unwrap();
    b.insert_rows("InstallUISequence", vec![
        vec![Value::from("CostInitialize"), Value::Null, Value::from(800i32)],
        vec![Value::from("CostFinalize"), Value::Null, Value::from(1000i32)],
    ]).unwrap();

    // === Add embedded cabinet as a stream ===
    b.add_stream("velo.cab".to_string(), cabinet);

    // Build the MSI
    let msi_data = b.build().unwrap();
    let msi_path = format!("{}\\velocity_file_test.msi", install_dir);
    std::fs::write(&msi_path, &msi_data).unwrap();
    println!("MSI written to: {} ({} bytes)", msi_path, msi_data.len());

    // Test with msiexec
    let log_path = format!("{}\\install.log", install_dir);
    println!("\n--- Installing ---");
    let status = Command::new("msiexec")
        .args(&[
            "/i", &msi_path,
            "/qn", "/norestart",
            "/l*v", &log_path,
            &format!("TARGETDIR={}", target_dir),
        ])
        .status();

    let code = match status {
        Ok(s) => s.code().unwrap_or(-1),
        Err(e) => {
            println!("Failed to run msiexec: {}", e);
            -1
        }
    };
    println!("msiexec /i exit code: {}", code);

    if code == 0 {
        // Check if file was installed
        let expected_file = format!("{}\\VelocityTest\\{}", target_dir, test_file_name);
        if std::path::Path::new(&expected_file).exists() {
            println!("SUCCESS: File installed at {}", expected_file);
            let content = std::fs::read_to_string(&expected_file).unwrap();
            println!("Content: {}", content);
        } else {
            println!("WARNING: msiexec returned 0 but file not found at {}", expected_file);
            // List what's in target dir
            if let Ok(entries) = std::fs::read_dir(target_dir) {
                for entry in entries.flatten() {
                    println!("  Found: {}", entry.path().display());
                }
            }
        }

        // Try to uninstall
        println!("\n--- Uninstalling ---");
        let status = Command::new("msiexec")
            .args(&["/x", &msi_path, "/qn", "/norestart"])
            .status();
        let uninst_code = match status {
            Ok(s) => s.code().unwrap_or(-1),
            Err(_) => -1,
        };
        println!("msiexec /x exit code: {}", uninst_code);
    } else {
        // Read the log for error details
        println!("\n--- Last 50 lines of install log ---");
        if let Ok(log) = std::fs::read_to_string(&log_path) {
            let lines: Vec<&str> = log.lines().collect();
            let start = lines.len().saturating_sub(50);
            for line in &lines[start..] {
                println!("{}", line);
            }
        }
    }
}
