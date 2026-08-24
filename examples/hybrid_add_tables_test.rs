/// Test: Add Media/File tables to a working velocity-msi MSI using the msi crate
/// cargo run --example hybrid_add_tables_test -p velocity-msi
use std::io::{Cursor, Read, Write};
use velocity_msi::{Column, MsiBuilder, Value};

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
    println!("=== HYBRID: velocity-msi + msi crate table addition ===\n");

    let pc = make_uuid();
    let uc = make_uuid();

    // Step 1: Create a working MSI with velocity-msi (Directory/Component/Feature tables)
    let mut builder = MsiBuilder::new();
    builder.set_title("Hybrid Test");
    builder.set_author("Velocity");
    builder.set_template("Intel", 1033);

    // Property table
    builder.create_table("Property", vec![
        Column::build("Property").string(72).primary_key().build(),
        Column::build("Value").nullable().string(255).localizable().build(),
    ]).unwrap();
    builder.insert_rows("Property", vec![
        vec![Value::from("ProductName"), Value::from("Hybrid Test")],
        vec![Value::from("ProductVersion"), Value::from("1.0.0")],
        vec![Value::from("Manufacturer"), Value::from("Velocity")],
        vec![Value::from("ProductCode"), Value::from(pc.as_str())],
        vec![Value::from("UpgradeCode"), Value::from(uc.as_str())],
        vec![Value::from("ProductLanguage"), Value::from("1033")],
    ]).unwrap();

    // Directory table
    builder.create_table("Directory", vec![
        Column::build("Directory").string(72).primary_key().build(),
        Column::build("Directory_Parent").nullable().string(72).build(),
        Column::build("DefaultDir").nullable().string(255).localizable().build(),
    ]).unwrap();
    builder.insert_rows("Directory", vec![
        vec![Value::from("TARGETDIR"), Value::Null, Value::from("SourceDir")],
        vec![Value::from("ProgramFilesFolder"), Value::from("TARGETDIR"), Value::from("PFiles")],
        vec![Value::from("INSTALLDIR"), Value::from("ProgramFilesFolder"), Value::from("VelHybrid")],
    ]).unwrap();

    // Component table
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

    // Feature table
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

    // FeatureComponents table
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

    // Build the base MSI
    let base_msi = builder.build().unwrap();
    println!("Base velocity-msi: {} bytes", base_msi.len());

    // Test base MSI (should work - exit 0)
    let base_path = "C:\\temp\\hybrid_base.msi";
    let _ = std::fs::remove_file(base_path);
    std::fs::write(base_path, &base_msi).unwrap();
    let output = std::process::Command::new("msiexec")
        .args(&["/i", base_path, "/qn"]).output().unwrap();
    let base_ec = output.status.code().unwrap_or(-1);
    println!("Base MSI exit code: {} (expect 0)", base_ec);
    let _ = std::process::Command::new("msiexec")
        .args(&["/x", &pc, "/qn"]).output();

    // Step 2: Open with msi crate and add File + Media tables + cabinet stream
    println!("\nAdding File/Media tables via msi crate...");
    let mut buf = base_msi.clone();
    {
        let cursor = Cursor::new(&mut buf);
        let mut package = msi::Package::open(cursor).expect("open with msi crate");

        // Add File table
        package.create_table("File", vec![
            msi::Column::build("File_").primary_key().id_string(72),
            msi::Column::build("Component_").foreign_key("Component", 1).id_string(72),
            msi::Column::build("FileName").formatted_string(255),
            msi::Column::build("FileSize").nullable().int32(),
            msi::Column::build("Sequence").int16(),
        ]).expect("create File");

        let file_content = b"Hello from hybrid test!";
        package.insert_rows(msi::Insert::into("File")
            .row(vec![
                msi::Value::from("MainFile"),
                msi::Value::from("MainComp"),
                msi::Value::from("testfile.txt"),
                msi::Value::Int(file_content.len() as i32),
                msi::Value::Int(1),
            ])
        ).expect("insert File");

        // Add Media table
        package.create_table("Media", vec![
            msi::Column::build("DiskId").primary_key().int16(),
            msi::Column::build("LastSequence").int16(),
            msi::Column::build("DiskPrompt").nullable().formatted_string(64),
            msi::Column::build("Cabinet").nullable().category(msi::Category::Cabinet).string(255),
            msi::Column::build("VolumeLabel").nullable().formatted_string(32),
            msi::Column::build("Source").nullable().formatted_string(72),
        ]).expect("create Media");

        package.insert_rows(msi::Insert::into("Media")
            .row(vec![
                msi::Value::Int(1),
                msi::Value::Int(1),
                msi::Value::Null,
                msi::Value::from("#velcab.cab"),
                msi::Value::Null,
                msi::Value::Null,
            ])
        ).expect("insert Media");

        // Add cabinet stream
        let cab_files = vec![
            velocity_msi::CabinetFile {
                name: "testfile.txt".to_string(),
                data: file_content.to_vec(),
            },
        ];
        let cab_data = velocity_msi::build_cabinet(&cab_files);
        println!("Cabinet data: {} bytes", cab_data.len());

        {
            let mut writer = package.write_stream("#velcab.cab").expect("write_stream");
            writer.write_all(&cab_data).expect("write cab");
        }

        package.flush().expect("flush");
    }

    println!("Modified MSI: {} bytes", buf.len());

    // Test modified MSI
    let mod_path = "C:\\temp\\hybrid_modified.msi";
    let log_path = "C:\\temp\\hybrid_modified.log";
    let _ = std::fs::remove_file(mod_path);
    let _ = std::fs::remove_file(log_path);
    std::fs::write(mod_path, &buf).unwrap();

    println!("\nTesting modified MSI with msiexec...");
    let output = std::process::Command::new("msiexec")
        .args(&["/i", mod_path, "/qn", "/l*v", log_path])
        .output().unwrap();
    let mod_ec = output.status.code().unwrap_or(-1);
    println!("Modified MSI exit code: {}", mod_ec);

    if mod_ec == 0 {
        println!("SUCCESS! Files installed!");
        let install_dir = "C:\\Program Files (x86)\\VelHybrid";
        if std::path::Path::new(install_dir).exists() {
            if let Ok(entries) = std::fs::read_dir(install_dir) {
                for entry in entries.flatten() {
                    println!("  INSTALLED: {} ({} bytes)",
                        entry.file_name().to_string_lossy(),
                        entry.metadata().map(|m| m.len()).unwrap_or(0));
                }
            }
        }
        let _ = std::process::Command::new("msiexec")
            .args(&["/x", &pc, "/qn"]).output();
    } else {
        println!("FAILED! Exit code: {}", mod_ec);
        if let Ok(log) = std::fs::read_to_string(log_path) {
            println!("\nKey log entries:");
            for line in log.lines() {
                if line.contains("error") || line.contains("Error") ||
                   line.contains("2725") || line.contains("1603") ||
                   line.contains("return value 3") {
                    println!("  {}", line);
                }
            }
        }
    }

    println!("\n=== RESULTS ===");
    println!("Base MSI:     {} (exit {})", if base_ec == 0 { "PASS" } else { "FAIL" }, base_ec);
    println!("Modified MSI: {} (exit {})", if mod_ec == 0 { "PASS" } else { "FAIL" }, mod_ec);
}
