/// Definitive install test: all required tables + embedded cabinet.
/// Goal: msiexec /i exits with code 0 AND files are written to disk.
use velocity_msi::{MsiBuilder, Column, Value, CabinetFile, build_cabinet};

fn main() {
    let content = b"Hello from Velocity MSI!\r\n";

    let mut builder = MsiBuilder::new();
    builder.set_title("Velocity Install Test");
    builder.set_author("Velocity");
    builder.set_template("Intel", 1033);

    // ── Property table ──────────────────────────────────────────
    builder.create_table("Property", vec![
        Column::build("Property").string(72).primary_key().build(),
        Column::build("Value").string(1024).build(),
    ]).unwrap();

    let product_code = "{AADDD4E4-3A12-4B7D-A4E0-5A2F8B3C6D90}";
    let upgrade_code = "{BBDDE5F5-4B23-5C8E-B5F1-6B3A9C4D7EA1}";

    builder.insert_rows("Property", vec![
        vec![Value::from("ProductName"), Value::from("Velocity Test")],
        vec![Value::from("ProductVersion"), Value::from("1.0.0")],
        vec![Value::from("Manufacturer"), Value::from("Velocity Corp")],
        vec![Value::from("ProductCode"), Value::from(product_code)],
        vec![Value::from("UpgradeCode"), Value::from(upgrade_code)],
        vec![Value::from("ProductLanguage"), Value::from("1033")],
    ]).unwrap();

    // ── Directory table ─────────────────────────────────────────
    builder.create_table("Directory", vec![
        Column::build("Directory").string(72).primary_key().build(),
        Column::build("Directory_Parent").string(72).nullable().build(),
        Column::build("DefaultDir").string(255).primary_key().build(),
    ]).unwrap();
    builder.insert_rows("Directory", vec![
        vec![Value::from("TARGETDIR"), Value::Null, Value::from("SourceDir")],
        vec![Value::from("INSTALLDIR"), Value::from("TARGETDIR"), Value::from("VelTest:VelTest")],
    ]).unwrap();

    // ── Component table ─────────────────────────────────────────
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

    // ── File table (8 columns) ──────────────────────────────────
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

    // ── Feature table ───────────────────────────────────────────
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
        vec![Value::from("Complete"), Value::Null, Value::from("Complete Install"),
             Value::from("Installs all files"), Value::Int(1), Value::Int(1),
             Value::Null, Value::Null],
    ]).unwrap();

    // ── FeatureComponents table ─────────────────────────────────
    builder.create_table("FeatureComponents", vec![
        Column::build("Feature_").string(38).primary_key().build(),
        Column::build("Component_").string(72).primary_key().build(),
    ]).unwrap();
    builder.insert_rows("FeatureComponents", vec![
        vec![Value::from("Complete"), Value::from("MainComp")],
    ]).unwrap();

    // ── Media table ─────────────────────────────────────────────
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
    // '#' prefix means embedded in the MSI package
    builder.insert_rows("Media", vec![
        vec![Value::Int(1), Value::Int(1), Value::Null,
             Value::from("#vel.cab"), Value::Null, Value::Null],
    ]).unwrap();
    builder.add_stream("vel.cab".to_string(), cab);

    // ── InstallExecuteSequence table ────────────────────────────
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

    // ── InstallUISequence table ─────────────────────────────────
    builder.create_table("InstallUISequence", vec![
        Column::build("Action").string(72).primary_key().build(),
        Column::build("Condition").string(255).nullable().build(),
        Column::build("Sequence").int16().nullable().build(),
    ]).unwrap();
    builder.insert_rows("InstallUISequence", vec![
        vec![Value::from("CostFinalize"), Value::Null, Value::Int(1000)],
        vec![Value::from("CostInitialize"), Value::Null, Value::Int(800)],
        vec![Value::from("ExecuteAction"), Value::Null, Value::Int(1300)],
        vec![Value::from("FileCost"), Value::Null, Value::Int(900)],
    ]).unwrap();

    // ── Build and write MSI ─────────────────────────────────────
    let msi = builder.build().unwrap();
    std::fs::write("velocity_install_test.msi", &msi).unwrap();
    println!("MSI written: {} bytes", msi.len());

    // Clean up any previous install
    let _ = std::fs::remove_dir_all("C:\\VelTest");

    // ── Test with msiexec ───────────────────────────────────────
    println!("\n=== Installing ===");
    let output = std::process::Command::new("msiexec")
        .args(&["/i", "velocity_install_test.msi", "/qn", "/l*v", "install_log.txt"])
        .output().unwrap();
    let exit_code = output.status.code().unwrap_or(-1);
    println!("msiexec /i exit code: {}", exit_code);

    // Print errors from log
    if let Ok(log) = std::fs::read_to_string("install_log.txt") {
        for line in log.lines() {
            if (line.contains("Error ") && !line.contains("Error 0"))
                || line.contains("return value 3")
                || line.contains("cabinet")
                || line.contains("Cabinet")
            {
                println!("  LOG: {}", line.trim());
            }
        }
    }

    // ── Check if files were installed ───────────────────────────
    let installed_path = "C:\\VelTest\\hello.txt";
    if std::path::Path::new(installed_path).exists() {
        let data = std::fs::read_to_string(installed_path).unwrap();
        println!("\nSUCCESS: File installed at {}", installed_path);
        println!("Content: {:?}", data);
    } else {
        println!("\nFAIL: File NOT found at {}", installed_path);
        if let Ok(entries) = std::fs::read_dir("C:\\VelTest") {
            println!("Contents of C:\\VelTest:");
            for entry in entries {
                if let Ok(e) = entry {
                    println!("  {}", e.path().display());
                }
            }
        } else {
            println!("C:\\VelTest directory does not exist");
        }
    }

    // ── Test uninstall ──────────────────────────────────────────
    if exit_code == 0 {
        println!("\n=== Uninstalling ===");
        let output = std::process::Command::new("msiexec")
            .args(&["/x", product_code, "/qn", "/l*v", "uninstall_log.txt"])
            .output().unwrap();
        println!("msiexec /x exit code: {}", output.status.code().unwrap_or(-1));

        if !std::path::Path::new(installed_path).exists() {
            println!("SUCCESS: File removed after uninstall");
        } else {
            println!("FAIL: File still exists after uninstall");
        }
    }
}
