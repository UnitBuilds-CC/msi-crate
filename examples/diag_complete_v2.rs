/// Complete MSI with all required tables including InstallExecuteSequence
/// cargo run --example diag_complete_v2 -p velocity-msi
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

    let file_content = b"Hello from velocity-msi installer!\r\n";
    let file_name = "velocity_test.txt";

    let cabinet = build_cabinet(&[
        CabinetFile {
            name: file_name.to_string(),
            data: file_content.to_vec(),
        },
    ]);
    println!("Cabinet size: {} bytes", cabinet.len());

    let mut b = MsiBuilder::new();
    b.set_title("Velocity Test V2");
    b.set_author("Velocity");
    b.set_template("Intel", 1033);

    // === Property table ===
    b.create_table("Property", vec![
        Column::build("Property").string(72).primary_key().build(),
        Column::build("Value").nullable().string(255).localizable().build(),
    ]).unwrap();
    b.insert_rows("Property", vec![
        vec![Value::from("ProductName"), Value::from("Velocity Test V2")],
        vec![Value::from("ProductVersion"), Value::from("1.0.0")],
        vec![Value::from("Manufacturer"), Value::from("Velocity")],
        vec![Value::from("ProductCode"), Value::from(product_code.as_str())],
        vec![Value::from("UpgradeCode"), Value::from(upgrade_code.as_str())],
        vec![Value::from("ProductLanguage"), Value::from("1033")],
    ]).unwrap();

    // === Directory table ===
    b.create_table("Directory", vec![
        Column::build("Directory").string(72).primary_key().build(),
        Column::build("Directory_Parent").nullable().string(72).build(),
        Column::build("DefaultDir").string(255).localizable().build(),
    ]).unwrap();
    b.insert_rows("Directory", vec![
        vec![Value::from("TARGETDIR"), Value::Null, Value::from("C:\\Program Files\\VelocityV2")],
        vec![Value::from("INSTALLDIR"), Value::from("TARGETDIR"), Value::from("VelocityV2:.")],
    ]).unwrap();

    // === Component table ===
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
            Value::from("MainComp"),
            Value::from(component_code.as_str()),
            Value::from("INSTALLDIR"),
            Value::Int(0),
            Value::Null,
            Value::from("MainFile"),
        ],
    ]).unwrap();

    // === Feature table ===
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

    // === FeatureComponents table ===
    b.create_table("FeatureComponents", vec![
        Column::build("Feature_").string(38).primary_key().build(),
        Column::build("Component_").string(72).primary_key().build(),
    ]).unwrap();
    b.insert_rows("FeatureComponents", vec![
        vec![Value::from("Complete"), Value::from("MainComp")],
    ]).unwrap();

    // === File table ===
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
            Value::from("MainComp"),
            Value::from(file_name),
            Value::Int(file_content.len() as i32),
            Value::Int(0),
            Value::Int(1),
        ],
    ]).unwrap();

    // === Media table ===
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
            Value::Int(1),
            Value::from("#velocity.cab"),
            Value::Null,
            Value::Null,
        ],
    ]).unwrap();

    // === InstallExecuteSequence table ===
    // Standard action sequence for a minimal install
    b.create_table("InstallExecuteSequence", vec![
        Column::build("Action").string(72).primary_key().build(),
        Column::build("Condition").nullable().string(255).build(),
        Column::build("Sequence").int16().build(),
    ]).unwrap();
    b.insert_rows("InstallExecuteSequence", vec![
        vec![Value::from("LaunchConditions"), Value::Null, Value::Int(100)],
        vec![Value::from("FindRelatedProducts"), Value::Null, Value::Int(200)],
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

    // === InstallUISequence table ===
    b.create_table("InstallUISequence", vec![
        Column::build("Action").string(72).primary_key().build(),
        Column::build("Condition").nullable().string(255).build(),
        Column::build("Sequence").int16().build(),
    ]).unwrap();
    b.insert_rows("InstallUISequence", vec![
        vec![Value::from("LaunchConditions"), Value::Null, Value::Int(100)],
        vec![Value::from("FindRelatedProducts"), Value::Null, Value::Int(200)],
        vec![Value::from("CostInitialize"), Value::Null, Value::Int(800)],
        vec![Value::from("FileCost"), Value::Null, Value::Int(900)],
        vec![Value::from("CostFinalize"), Value::Null, Value::Int(1000)],
        vec![Value::from("ExecuteAction"), Value::Null, Value::Int(1300)],
    ]).unwrap();

    // Embed cabinet
    b.add_stream("velocity.cab".to_string(), cabinet.clone());

    let msi_data = b.build().unwrap();
    println!("MSI size: {} bytes", msi_data.len());

    let _ = std::fs::create_dir_all("C:\\temp");
    let path = "C:\\temp\\test_complete_v2.msi";
    let log_path = "C:\\temp\\test_complete_v2.log";
    let _ = std::fs::remove_file(path);
    let _ = std::fs::remove_file(log_path);
    std::fs::write(path, &msi_data).unwrap();

    let output = std::process::Command::new("msiexec")
        .args(&["/i", path, "/qn", "/l*v", log_path])
        .output().unwrap();
    let ec = output.status.code().unwrap_or(-1);
    println!("Exit code: {}", ec);

    if ec == 0 {
        println!("\nSUCCESS! MSI installed!");
        let install_path = "C:\\Program Files\\VelocityV2\\velocity_test.txt";
        if std::path::Path::new(install_path).exists() {
            let content = std::fs::read_to_string(install_path).unwrap();
            println!("File content: {:?}", content);
        } else {
            println!("File NOT found at: {}", install_path);
            // Search for installed files
            if let Ok(entries) = std::fs::read_dir("C:\\Program Files\\VelocityV2") {
                for entry in entries.flatten() {
                    println!("  Found: {:?}", entry.path());
                }
            }
        }
    } else {
        if let Ok(log) = std::fs::read_to_string(log_path) {
            let lines: Vec<&str> = log.lines().collect();
            let start = lines.len().saturating_sub(120);
            println!("\n--- Last 120 lines of log ---");
            for line in &lines[start..] {
                println!("{}", line);
            }
        }
    }
}
