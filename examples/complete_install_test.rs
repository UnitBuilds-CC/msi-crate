/// Complete installable MSI test with Directory/Component/File/Feature/Media tables
/// and an embedded cabinet file. Tests with msiexec.
use velocity_msi::{MsiBuilder, Column, Value, CabinetFile, build_cabinet};

fn main() {
    let mut builder = MsiBuilder::new();
    builder.set_title("Velocity Test Product");
    builder.set_author("Velocity Installer");
    builder.set_template("x64", 1033);

    // --- Property table ---
    builder.create_table("Property", vec![
        Column::build("Property").string(72).primary_key().build(),
        Column::build("Value").string(255).nullable().build(),
    ]).unwrap();

    let product_code = "{12345678-1234-1234-1234-123456789ABC}";
    let upgrade_code = "{ABCDEFGH-ABCD-ABCD-ABCD-ABCDEFGHIJKL}";
    // Fix: use proper GUID format
    let upgrade_code = "{87654321-4321-4321-4321-CBA987654321}";

    builder.insert_rows("Property", vec![
        vec![Value::from("ProductName"), Value::from("Velocity Test Product")],
        vec![Value::from("ProductVersion"), Value::from("1.0.0")],
        vec![Value::from("Manufacturer"), Value::from("Velocity Corp")],
        vec![Value::from("ProductCode"), Value::from(product_code)],
        vec![Value::from("UpgradeCode"), Value::from(upgrade_code)],
        vec![Value::from("ProductLanguage"), Value::from("1033")],
    ]).unwrap();

    // --- Directory table ---
    builder.create_table("Directory", vec![
        Column::build("Directory").string(72).primary_key().build(),
        Column::build("Directory_Parent").string(72).nullable().build(),
        Column::build("DefaultDir").string(255).primary_key().build(),
    ]).unwrap();

    builder.insert_rows("Directory", vec![
        // TARGETDIR is the implicit root
        vec![Value::from("TARGETDIR"), Value::Null, Value::from("SourceDir")],
        // ProgramFiles64Folder under TARGETDIR
        vec![Value::from("ProgramFiles64Folder"), Value::from("TARGETDIR"), Value::from("PFiles")],
        // Our install directory under ProgramFiles64Folder
        vec![Value::from("INSTALLDIR"), Value::from("ProgramFiles64Folder"), Value::from("VelocityTest")],
    ]).unwrap();

    // --- Component table ---
    builder.create_table("Component", vec![
        Column::build("Component").string(72).primary_key().build(),
        Column::build("ComponentId").string(38).nullable().build(),
        Column::build("Directory_").string(72).build(),
        Column::build("Attributes").int16().build(),
        Column::build("Condition").string(255).nullable().build(),
        Column::build("KeyPath").string(72).nullable().build(),
    ]).unwrap();

    builder.insert_rows("Component", vec![
        vec![
            Value::from("MainComponent"),
            Value::from("{AAAAAAAA-AAAA-AAAA-AAAA-AAAAAAAAAAAA}"),
            Value::from("INSTALLDIR"),
            Value::Int(0), // 0 = win64=no (use 256 for x64)
            Value::Null,
            Value::from("test_file.txt"), // KeyPath = File name
        ],
    ]).unwrap();

    // --- File table ---
    builder.create_table("File", vec![
        Column::build("File").string(72).primary_key().build(),
        Column::build("Component_").string(72).build(),
        Column::build("FileName").string(255).build(),
        Column::build("FileSize").int32().build(),
        Column::build("Sequence").int16().build(),
    ]).unwrap();

    let test_content = b"Hello from Velocity Installer!\r\nThis is a test file.\r\n";
    builder.insert_rows("File", vec![
        vec![
            Value::from("test_file.txt"),
            Value::from("MainComponent"),
            Value::from("test_file.txt"),
            Value::Int(test_content.len() as i32),
            Value::Int(1),
        ],
    ]).unwrap();

    // --- Feature table ---
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
        vec![
            Value::from("Complete"),
            Value::Null,
            Value::from("Complete Installation"),
            Value::from("Install all files"),
            Value::Int(1),
            Value::Int(1),
            Value::from("INSTALLDIR"),
            Value::Null,
        ],
    ]).unwrap();

    // --- FeatureComponents table ---
    builder.create_table("FeatureComponents", vec![
        Column::build("Feature_").string(38).primary_key().build(),
        Column::build("Component_").string(72).primary_key().build(),
    ]).unwrap();

    builder.insert_rows("FeatureComponents", vec![
        vec![
            Value::from("Complete"),
            Value::from("MainComponent"),
        ],
    ]).unwrap();

    // --- Media table ---
    builder.create_table("Media", vec![
        Column::build("DiskId").int16().primary_key().build(),
        Column::build("LastSequence").int16().build(),
        Column::build("DiskPrompt").string(64).nullable().build(),
        Column::build("Cabinet").string(255).nullable().build(),
        Column::build("VolumeLabel").string(32).nullable().build(),
        Column::build("Source").string(72).nullable().build(),
    ]).unwrap();

    // Build cabinet with the test file
    let cab_data = build_cabinet(&[
        CabinetFile {
            name: "test_file.txt".to_string(),
            data: test_content.to_vec(),
        },
    ]);

    // The '#' prefix means embedded cabinet
    builder.insert_rows("Media", vec![
        vec![
            Value::Int(1),
            Value::Int(1),
            Value::Null,
            Value::from("#velocity.cab"),
            Value::Null,
            Value::Null,
        ],
    ]).unwrap();

    // Add the cabinet as an embedded stream
    builder.add_stream("velocity.cab".to_string(), cab_data);

    // Build the MSI
    let msi_data = builder.build().unwrap();
    std::fs::write("complete_test.msi", &msi_data).unwrap();
    println!("MSI size: {} bytes", msi_data.len());

    // Test with msiexec
    println!("\n=== msiexec install test ===");
    let output = std::process::Command::new("msiexec")
        .args(&[
            "/i", "complete_test.msi",
            "/qn",
            "/l*v", "complete_test_log.txt",
            "TARGETDIR=C:\\VelocityTestOutput",
        ])
        .output()
        .expect("Failed to run msiexec");
    let code = output.status.code().unwrap_or(-1);
    println!("Install exit code: {}", code);

    // Check log for errors
    if let Ok(log) = std::fs::read_to_string("complete_test_log.txt") {
        let mut found_errors = false;
        for line in log.lines() {
            if line.contains("returning") || line.contains("CustomAction") && line.contains("failed")
                || (line.contains("Error:") && !line.contains("0"))
                || line.contains("Note: 1:") {
                println!("  {}", line.trim());
                found_errors = true;
            }
        }
        if !found_errors {
            println!("  (no errors in log)");
        }
    }

    // Check if file was installed
    let installed_path = "C:\\VelocityTestOutput\\test_file.txt";
    if std::path::Path::new(installed_path).exists() {
        println!("\nSUCCESS: File installed at {}", installed_path);
        let content = std::fs::read_to_string(installed_path).unwrap();
        println!("Content: {:?}", &content[..content.len().min(50)]);

        // Try uninstall
        println!("\n=== msiexec uninstall test ===");
        let output = std::process::Command::new("msiexec")
            .args(&[
                "/x", product_code,
                "/qn",
                "/l*v", "complete_uninstall_log.txt",
            ])
            .output()
            .expect("Failed to run msiexec");
        let code = output.status.code().unwrap_or(-1);
        println!("Uninstall exit code: {}", code);

        // Check if file was removed
        if !std::path::Path::new(installed_path).exists() {
            println!("SUCCESS: File removed after uninstall");
        } else {
            println!("WARNING: File still exists after uninstall");
        }

        // Cleanup
        let _ = std::fs::remove_dir_all("C:\\VelocityTestOutput");
        for f in &["complete_uninstall_log.txt"] {
            let _ = std::fs::remove_file(f);
        }
    } else {
        println!("\nFile NOT installed at {}", installed_path);
        // Check alternate locations
        let alt_path = "C:\\Program Files\\VelocityTest\\test_file.txt";
        if std::path::Path::new(alt_path).exists() {
            println!("But file found at alternate location: {}", alt_path);
            let _ = std::fs::remove_dir_all("C:\\Program Files\\VelocityTest");
        }
    }

    // Cleanup
    for f in &["complete_test.msi", "complete_test_log.txt"] {
        let _ = std::fs::remove_file(f);
    }
}
